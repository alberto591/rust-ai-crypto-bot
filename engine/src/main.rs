use std::env;
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use dotenvy::dotenv;
use solana_sdk::signature::{read_keypair_file, Signer};
use solana_sdk::pubkey::Pubkey;
use tracing::{info, error, warn, debug};
// use futures_util::future;

// Internal Crates
use strategy::StrategyEngine;
// Removed unused JitoExecutor and LegacyExecutor

mod config;
mod listener;
mod pool_fetcher;
mod devnet_keys;
mod wallet_manager;
mod tui;
mod recorder;
mod metrics;
mod risk;
mod telemetry;
mod alerts;
mod intelligence;
mod discovery;
mod birth_watcher;
mod watcher;
mod scoring;

use crate::intelligence::MarketIntelligence;
use crate::wallet_manager::WalletManager;

/// Global Application Context
/// Shared, read-only resources wired together at startup
pub struct AppContext {
    pub config: config::BotConfig,
    pub payer: solana_sdk::signature::Keypair,
    pub engine: Arc<StrategyEngine>,
    pub wallet_mgr: Arc<WalletManager>,
    pub performance_tracker: Arc<strategy::analytics::performance::PerformanceTracker>,
    pub metrics: Arc<metrics::BotMetrics>,
    pub risk_mgr: Arc<risk::RiskManager>,
    pub alert_mgr: Arc<alerts::AlertManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let bot_start_time = tokio::time::Instant::now();
    
    // 1. Initial Logging Setup (Plaintext for bootstrap)
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
        )
        .init();
    
    info!("🚀 HFT Engine Bootstrapping [Composition Root]...");

    // 3. Unified Configuration Layer
    let bot_cfg: config::BotConfig = match config::BotConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("❌ CRITICAL: Failed to load config: {}", e);
            std::process::exit(1);
        }
    };
    
    // 4. Startup Validation (Fail Fast)
    if let Err(e) = bot_cfg.validate() {
        error!("❌ Configuration Validation Failed: {}", e);
        std::process::exit(1);
    }
    
    // 4.1 Initialize Data Recorder (Ops Layer)
    let recording_enabled = env::var("DATA_RECORDING_ENABLED").unwrap_or_else(|_| "false".to_string()) == "true";
    let recorder = if recording_enabled {
        info!("💾 Data Recording ENABLED. Initializing recorder...");
        match recorder::AsyncCsvWriter::new("data").await {
            Ok(r) => Some(Arc::new(r)),
            Err(e) => {
                error!("❌ Failed to initialize Data Recorder: {}", e);
                None
            }
        }
    } else {
        info!("🚫 Data Recording DISABLED.");
        None
    };
    
    info!("✅ Config Loaded & Validated: RPC={}, Jito={}", bot_cfg.rpc_url, bot_cfg.jito_url);
    
    let key_path = if bot_cfg.keypair_path.is_empty() {
        format!("{}/.config/solana/id.json", env::var("HOME").unwrap_or_else(|_| ".".to_string()))
    } else {
        bot_cfg.keypair_path.clone()
    };
    
    let payer = match read_keypair_file(&key_path) {
        Ok(k) => k,
        Err(e) => {
            error!("❌ CRITICAL: Failed to read keypair at {}: {}", key_path, e);
            std::process::exit(1);
        }
    };
    info!("🔑 Identity: {}", payer.pubkey());

    // --- COMPOSITION ROOT SETUP ---
    
    // 1. Initialize Database & Market Intelligence FIRST (Phase 3 Hardening)
    let db_pool = if let Ok(db_url) = std::env::var("DATABASE_URL") {
        let pg_config = tokio_postgres::Config::from_str(&db_url);
        match pg_config {
            Ok(conf) => {
                let mgr = deadpool_postgres::Manager::new(conf, tokio_postgres::NoTls);
                let pool = deadpool_postgres::Pool::builder(mgr)
                    .max_size(5)
                    .build();
                match pool {
                    Ok(p) => Some(p),
                    Err(e) => {
                        error!("❌ Failed to build Postgres Pool: {}. Falling back to file storage.", e);
                        None
                    }
                }
            },
            Err(e) => {
                error!("❌ Invalid DATABASE_URL: {}. Falling back to file storage.", e);
                None
            }
        }
    } else {
        warn!("⚠️ DATABASE_URL not set. Success Library will use file fallback.");
        None
    };

    let intel_impl = Arc::new(intelligence::DatabaseIntelligence::new(db_pool.clone()));
    let intel_port: Arc<dyn strategy::ports::MarketIntelligencePort> = Arc::clone(&intel_impl) as Arc<dyn strategy::ports::MarketIntelligencePort>;
    let intelligence_mgr: Arc<dyn MarketIntelligence> = Arc::clone(&intel_impl) as Arc<dyn MarketIntelligence>;
    let scoring_engine = Arc::new(scoring::PoolScoringEngine::new(db_pool.clone()));

    // 1.1 Initialize Scoring DB & Load Weights
    if let Err(e) = scoring_engine.init_db().await {
        error!("❌ Failed to initialize scoring DB: {}", e);
    }
    if let Err(e) = scoring_engine.load_from_db().await {
        error!("❌ Failed to load scores from DB: {}", e);
    }

    // 2. Initialize Telemetry & Metrics (with Intelligence reference)
    info!("🔌 Connecting to RPC: {}...", bot_cfg.rpc_url);
    let metrics = Arc::new(metrics::BotMetrics::new(Some(Arc::clone(&intel_port))));
    let pool_fetcher = Arc::new(pool_fetcher::PoolKeyFetcher::new(&bot_cfg.rpc_url));
    let risk_mgr = Arc::new(risk::RiskManager::new());

    // 4.3 Initialize Performance & Safety
    info!("📊 Initializing Performance Tracker...");
    let performance_tracker = Arc::new(strategy::analytics::performance::PerformanceTracker::new("logs/performance.log").await);
    info!("🛡️ Initializing Safety Checker...");
    let safety_checker = Arc::new(strategy::safety::token_validator::TokenSafetyChecker::new(&bot_cfg.rpc_url, bot_cfg.min_liquidity_lamports));

    // 4.4 Initialize Execution Engine (Abstracted)
    info!("⚡ Initializing Execution Port (Jito preference)...");
    let execution_port: Arc<dyn strategy::ports::ExecutionPort> = if bot_cfg.jito_url.is_empty() {
        info!("⚠️ Jito URL empty. Falling back to Legacy RPC Executor.");
        Arc::new(executor::legacy::LegacyExecutor::new(
            &bot_cfg.rpc_url,
            solana_sdk::signature::Keypair::from_bytes(&payer.to_bytes()).map_err(|e| anyhow::anyhow!("Keypair clone failed: {}", e))?,
            Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
        ))
    } else {
        match executor::jito::JitoExecutor::new(
            &bot_cfg.jito_url,
            &payer,
            &bot_cfg.rpc_url,
            bot_cfg.helius_sender_url.clone(),
            bot_cfg.fee_strategy.clone(),
            Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
            Some(Arc::clone(&metrics) as Arc<dyn strategy::ports::TelemetryPort>),
        ).await {
            Ok(jito) => Arc::new(jito),
            Err(e) => {
                warn!("❌ Jito initialization failed: {}. Falling back to Legacy.", e);
                Arc::new(executor::legacy::LegacyExecutor::new(
                    &bot_cfg.rpc_url,
                    solana_sdk::signature::Keypair::from_bytes(&payer.to_bytes()).map_err(|e| anyhow::anyhow!("Keypair clone failed: {}", e))?,
                    Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
                ))
            }
        }
    };
    

    // 4.5 Initialize Strategy Engine (The Brain)
    let ai_model = match strategy::adapters::ONNXModelAdapter::from_file("ai_model.onnx") {
        Ok(model) => {
            info!("🧠 AI Model loaded successfully (ai_model.onnx)");
            Some(Arc::new(model) as Arc<dyn strategy::ports::AIModelPort>)
        }
        Err(e) => {
            warn!("⚠️ Failed to load AI model: {}. Running in heuristic mode.", e);
            None
        }
    };

    let engine = Arc::new(StrategyEngine::new(
        Some(execution_port),
        None, // No simulation in prod
        ai_model,
        Some(Arc::clone(&performance_tracker)),
        Some(Arc::clone(&safety_checker)),
        Some(Arc::clone(&metrics) as Arc<dyn strategy::ports::TelemetryPort>),
        Some(intel_port),
    ));

    let wallet_mgr = Arc::new(WalletManager::new(&bot_cfg.rpc_url));
    
    // 4.6 Initialize Alerting
    let telegram_config = if let (Some(token), Some(chat_id)) = (&bot_cfg.telegram_bot_token, &bot_cfg.telegram_chat_id) {
        let token_str: String = token.clone();
        let chat_id_str: String = chat_id.clone();
        Some(alerts::TelegramConfig {
            bot_token: token_str,
            chat_id: chat_id_str,
        })
    } else {
        None
    };
    let alert_mgr = Arc::new(alerts::AlertManager::new(
        bot_cfg.discord_webhook.clone(), 
        telegram_config,
        bot_cfg.ntfy_topic.clone(),
    ));
    tracing::info!("🔔 Alerting configured: Discord={}, Telegram={}", 
        bot_cfg.discord_webhook.is_some(),
        bot_cfg.telegram_bot_token.is_some() && bot_cfg.telegram_chat_id.is_some()
    );

    // 4.3.6 Initialize Telemetry
    mev_core::telemetry::init_metrics();
    tokio::spawn(telemetry::serve_metrics());
    
    // Start health monitor (status checks every 5 minutes + hourly summary)
    tokio::spawn(alerts::monitor_health(
        Arc::clone(&alert_mgr), 
        Arc::clone(&metrics),
        Arc::clone(&wallet_mgr),
        payer.pubkey(),
        bot_start_time
    ));

    // Start Telegram Command Listener (V2)
    tokio::spawn(Arc::clone(&alert_mgr).handle_telegram_commands(
        Arc::clone(&metrics),
        Arc::clone(&wallet_mgr),
        payer.pubkey(),
        bot_start_time
    ));

    // Start 5-minute periodic weight sync (PostgreSQL)
    let scoring_engine_sync = Arc::clone(&scoring_engine);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if let Err(e) = scoring_engine_sync.sync_to_db().await {
                error!("❌ Failed to sync pool weights: {}", e);
            }
        }
    });

    // Start 5-minute periodic reporting (Log-based)
    let metrics_clone = Arc::clone(&metrics);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            metrics_clone.print_periodic_update();
        }
    });

    // 4.4 Assemble Context (Composition Root)
    let context = Arc::new(AppContext {
        config: bot_cfg.clone(),
        payer,
        engine,
        wallet_mgr,
        performance_tracker,
        metrics,
        risk_mgr,
        alert_mgr: Arc::clone(&alert_mgr),
    });

    // 4.5 Pre-flight Wallet Verification
    info!("🧪 Cooling down for RPC stability (3s)...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    info!("🧪 Validating Wallet state for monitored tokens...");
    let mut unique_mints = std::collections::HashSet::new();
    for pool in config::MONITORED_POOLS {
        unique_mints.insert(pool.token_a);
        unique_mints.insert(pool.token_b);
    }
    
    let unique_mints_vec: Vec<Pubkey> = unique_mints.into_iter().collect();
    
    match context.wallet_mgr.check_atas_exist(&context.payer.pubkey(), &unique_mints_vec).await {
        Ok(results) => {
            let mut missing_atas = Vec::new();
            for (mint, exists) in results {
                if !exists {
                    missing_atas.push(mint);
                }
            }
            if !missing_atas.is_empty() {
                info!("📦 Found {} missing ATAs. Preparing for lazy creation...", missing_atas.len());
            } else {
                info!("✅ All required ATAs exist.");
            }
        },
        Err(e) => warn!("⚠️ Failed to batch check ATAs: {}. Proceeding anyway.", e),
    }

    // 4.6 Pre-flight Balance Checks (Gas & Capital)
    info!("💰 Checking balances...");
    match context.wallet_mgr.get_sol_balance(&context.payer.pubkey()).await {
        Ok(balance) => {
            let sol = balance as f64 / 1e9;
            if balance < 100_000_000 { // 0.1 SOL
                warn!("⚠️ LOW SOL BALANCE: {:.4} SOL. Gas might run out during high activity.", sol);
            } else {
                info!("✅ SOL Balance: {:.4} SOL (Gas Safe)", sol);
            }
        }
        Err(e) => error!("❌ Failed to fetch real SOL balance: {}", e),
    }

    info!("📊 --- STARTUP TOKEN INVENTORY ---");
    let mut inventory_mints = unique_mints_vec.clone();
    inventory_mints.retain(|m| *m != mev_core::constants::SOL_MINT);

    match context.wallet_mgr.get_multiple_token_balances(&context.payer.pubkey(), &inventory_mints).await {
        Ok(balances) => {
            let mut inventory = std::collections::HashMap::new();
            for (mint, balance) in balances {
                let symbol = match mint {
                    mev_core::constants::USDC_MINT => "USDC",
                    mev_core::constants::JUP_MINT => "JUP ",
                    mev_core::constants::RAY_MINT => "RAY ",
                    mev_core::constants::BONK_MINT => "BONK",
                    mev_core::constants::WIF_MINT => "WIF ",
                    _ => "UNKN",
                };
                info!("   ├─ {}: {:.6} (raw: {})", symbol, balance as f64 / 1e6, balance);
                inventory.insert(symbol, balance);
            }
            info!("   └─ Total: {} tokens tracked", inventory.len());
        },
        Err(e) => error!("❌ Failed to batch fetch token balances: {}", e),
    }
    info!("📊 -------------------------------");
    
    let (tx, _rx) = tokio::sync::broadcast::channel::<mev_core::MarketUpdate>(1024);
    let (shutdown_tx, _shutdown_rx) = mpsc::channel::<()>(1);
    
    // 6.5. TUI Dashboard (Real-time Monitoring) - MOVED UP
    let no_tui = env::args().any(|a| a == "--no-tui");
    let tui_state = Arc::new(std::sync::Mutex::new(tui::AppState::new()));
    if !no_tui {
        let tui_state_clone = Arc::clone(&tui_state);
        std::thread::spawn(move || {
            if let Err(e) = tui::TuiApp::new(tui_state_clone).run() {
                error!("TUI error: {}", e);
            }
        });
        info!("📊 TUI Dashboard ACTIVE (press 'q' to quit)");
    }
    
    let mut pools_to_watch = HashMap::new();
    
    // 5. Initialize Monitored Pools (Priority: Static Roadmap List)
    for pool in config::MONITORED_POOLS {
        pools_to_watch.insert(
            pool.address.to_string(), 
            (pool.token_a.to_string(), pool.token_b.to_string())
        );
    }

    // Also include any pools from the .env if present (Merge)
    for addr in bot_cfg.monitored_pool_addresses.split(',') {
        let addr_str: &str = addr.trim();
        if !addr_str.is_empty() {
             pools_to_watch.entry(addr_str.to_string())
                 .or_insert_with(|| ("SOL".to_string(), "USDC".to_string()));
        }
    }

    // 5.5 Network Ingestion (Unified MarketWatcher)
    let (_sub_tx, sub_rx) = tokio::sync::mpsc::unbounded_channel();
    let (discovery_tx, discovery_rx) = mpsc::channel(128);
    
    let args: Vec<String> = env::args().collect();
    let discovery_enabled = args.contains(&"--discovery".to_string()) 
        || env::var("DISCOVERY_ENABLED").is_ok()
        || bot_cfg.mode != config::ExecutionMode::Simulation;
    let analyze_mode = args.contains(&"--analyze".to_string());

    let ws_url = bot_cfg.ws_url.clone();
    let rpc_url = bot_cfg.rpc_url.clone();
    let market_tx_watcher = tx.clone();
    let discovery_tx_watcher = discovery_tx.clone();
    let tui_watcher = Arc::clone(&tui_state);
    let monitored_pools = pools_to_watch.clone();

    let scoring_engine_watcher = Arc::clone(&scoring_engine);
    tokio::spawn(async move {
        watcher::start_market_watcher(
            ws_url,
            rpc_url,
            discovery_tx_watcher,
            market_tx_watcher,
            Some(tui_watcher),
            monitored_pools,
            sub_rx,
            scoring_engine_watcher,
        ).await;
    });

    // 6. Birth Watcher (New Pool Logic)
    if discovery_enabled {
        let birth_watcher = Arc::new(birth_watcher::BirthWatcher::new(
            Arc::new(bot_cfg.clone()),
            Arc::clone(&intelligence_mgr),
            &bot_cfg.rpc_url,
        ));
        
        tokio::spawn(async move {
            birth_watcher.run(discovery_rx).await;
        });
        info!("✅ Discovery & Birth Monitoring ACTIVE.");
    }

    // 6.1 Shutdown Watcher
    let shutdown_tx_signal = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("🛑 Shutdown signal received (Ctrl+C). Cleaning up...");
        let _ = shutdown_tx_signal.send(()).await;
    });

    // 6.4 Analysis Mode (Success DNA Extraction)
    if analyze_mode {
        info!("🧬 Analysis Mode Requested. Extracting Success DNA...");
        match intelligence_mgr.get_analysis().await {
            Ok(analysis) => {
                println!("\n🧬 ==========================================");
                println!("🧬   SUCCESS LIBRARY ANALYSIS (DNA REPORT)   ");
                println!("🧬 ==========================================");
                println!("🧬 Average Peak ROI:          {:.2}%", analysis.average_peak_roi);
                println!("🧬 Median Time to Peak:       {}s", analysis.median_time_to_peak);
                println!("🧬 Total Successful Launches: {}", analysis.total_successful_launches);
                println!("🧬 Strategy Effectiveness:    {:.2}%", analysis.strategy_effectiveness * 100.0);
                println!("🧬 ==========================================\n");
            },
            Err(e) => error!("❌ Failed to generate analysis: {}", e),
        }
    }

    // 6.5. TUI Dashboard (Real-time Monitoring) - MOVED TO STEP 6.1

    info!("🔥 Engine IGNITION. Waiting for market events...");

    // 6.6 Startup Alert
    alert_mgr.send_alert(
        alerts::AlertSeverity::Success, 
        "HFT Engine Started", 
        &format!("Engine version {} is now live. Monitoring {} pools.", env!("CARGO_PKG_VERSION"), pools_to_watch.len()),
        vec![
            alerts::Field { name: "Identity".to_string(), value: context.payer.pubkey().to_string(), inline: false },
            alerts::Field { name: "Jito".to_string(), value: (!bot_cfg.jito_url.is_empty()).to_string(), inline: true },
        ]
    ).await;
    
    // 7. Worker Pool Ignition (HFT Optimization)
    let num_workers = 8;
    for i in 0..num_workers {
        let mut worker_rx = tx.subscribe();
        let ctx = Arc::clone(&context);
        let rec_inner = recorder.clone();
        let tui_worker_clone = Arc::clone(&tui_state);
        
        tokio::spawn(async move {
            info!("👷 Worker {} started.", i);
            while let Ok(event) = worker_rx.recv().await {
                // Update WebSocket status in telemetry
                telemetry::WEBSOCKET_STATUS.set(1);

                // 🛡️ Remote Control Check
                if ctx.metrics.is_paused.load(std::sync::atomic::Ordering::Relaxed) {
                    continue;
                }

                let domain_update = Arc::new(mev_core::PoolUpdate {
                    pool_address: event.pool_address,
                    program_id: event.program_id,
                    mint_a: event.coin_mint,
                    mint_b: event.pc_mint,
                    reserve_a: event.coin_reserve as u128,
                    reserve_b: event.pc_reserve as u128,
                    price_sqrt: event.price_sqrt,
                    liquidity: event.liquidity,
                    fee_bps: 25, // Raydium V4 standard fee (0.25%) 
                    timestamp: event.timestamp as u64,
                });
                
                // Track discovery throughput if this is a new pool event
                // (Note: event is from listener, but discovery also sends events to birth_watcher)
                // Actually, let's track it in birth_watcher or discovery.rs directly.

                // Record Market Data
                if let Some(r) = &rec_inner {
                    let r_clone = Arc::clone(r);
                    let update_clone = Arc::clone(&domain_update);
                    tokio::spawn(async move {
                        r_clone.record((*update_clone).clone()).await;
                    });
                }

                // 🛡️ Risk Check
                if let Err(_e) = ctx.risk_mgr.can_trade(ctx.config.default_trade_size_lamports) {
                    continue; // Skip silently in hot path
                }

                let start_time = std::time::Instant::now();
                debug!("⏱️ START process_event at {:?}", start_time);
                let processing_result = ctx.engine.process_event(
                    domain_update, 
                    ctx.config.default_trade_size_lamports,
                    ctx.config.jito_tip_lamports,
                    ctx.config.jito_tip_percentage,
                    ctx.config.max_jito_tip_lamports,
                    ctx.config.max_slippage_bps,
                    ctx.config.volatility_sensitivity,
                    ctx.config.max_slippage_ceiling,
                    ctx.config.min_profit_threshold_lamports,
                    ctx.config.ai_confidence_threshold,
                    ctx.config.sanity_profit_factor,
                    ctx.config.max_hops
                ).await;
                
                let duration = start_time.elapsed().as_millis() as f64;
                debug!("⏱️ END process_event. Duration: {}ms", duration);
                telemetry::DETECTION_LATENCY.observe(duration);

                match processing_result {
                    Ok(Some(opportunity)) => {
                        telemetry::OPPORTUNITIES_TOTAL.inc();
                        telemetry::OPPORTUNITIES_PROFITABLE.inc();
                        
                        // Phase 11: DNA Telemetry
                        if opportunity.is_dna_match {
                            telemetry::DNA_MATCHES_TOTAL.inc();
                        }
                        if opportunity.is_elite_match {
                            telemetry::DNA_ELITE_MATCHES_TOTAL.inc();
                        }

                        ctx.metrics.log_opportunity(true);
                        
                        // Notify via Alerts
                        let am = Arc::clone(&ctx.alert_mgr);
                        let opp_clone = opportunity.clone();
                        tokio::spawn(async move {
                            am.send_trade_notification(&opp_clone, "Success (See Logs)").await;
                        });
                        
                        // Push to TUI
                        {
                            if let Ok(mut state) = tui_worker_clone.lock() {
                                state.recent_opportunities.push(opportunity.clone());
                                state.current_latency_ms = duration;
                                if opportunity.expected_profit_lamports > 0 {
                                    state.total_simulated_pnl += opportunity.expected_profit_lamports;
                                }
                            }
                        }

                        ctx.risk_mgr.record_trade(ctx.config.default_trade_size_lamports, opportunity.expected_profit_lamports as i64);
                        if let Some(r) = &rec_inner {
                            let _ = r.record_arbitrage(opportunity).await;
                        }
                    }
                    Ok(None) => {
                        telemetry::OPPORTUNITIES_TOTAL.inc();
                    }
                    Err(e) => {
                        telemetry::RPC_ERRORS.inc();
                        ctx.metrics.rpc_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        error!("💥 Worker {} processing error: {}", i, e);
                    }
                }
            }
        });
    }

    // --- GRACEFUL SHUTDOWN HANDLER ---
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("🛑 Received SIGINT (Ctrl+C). Initiating graceful shutdown...");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
                term.recv().await;
                info!("🛑 Received SIGTERM. Initiating graceful shutdown...");
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await;
            }
        } => {}
    }

    info!("👋 Engine shutting down gracefully...");
    let _ = scoring_engine.sync_to_db().await;
    context.metrics.print_summary();
    context.alert_mgr.send_final_report(Arc::clone(&context.metrics), bot_start_time).await;
    info!("Goodbye!");
    
    Ok(())
}
