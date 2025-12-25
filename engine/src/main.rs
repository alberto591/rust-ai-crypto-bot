use std::env;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use dotenvy::dotenv;
use solana_sdk::signature::{read_keypair_file, Signer};
use tracing::{info, error, warn};

// Internal Crates
use mev_core::MarketUpdate;
use strategy::{StrategyEngine};
use executor::jito::JitoExecutor;
use executor::legacy::LegacyExecutor;

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

use crate::wallet_manager::WalletManager;

/// Global Application Context
/// Shared, read-only resources wired together at startup
pub struct AppContext {
    pub config: config::BotConfig,
    pub payer: solana_sdk::signature::Keypair,
    pub engine: Arc<StrategyEngine>,
    pub wallet_mgr: WalletManager,
    pub performance_tracker: Arc<strategy::analytics::performance::PerformanceTracker>,
    pub metrics: Arc<metrics::BotMetrics>,
    pub risk_mgr: Arc<risk::RiskManager>,
    pub alert_mgr: Arc<alerts::AlertManager>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    
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
        format!("{}/.config/solana/id.json", env::var("HOME").unwrap())
    } else {
        bot_cfg.keypair_path.clone()
    };
    
    let payer = read_keypair_file(&key_path).expect("Failed to read keypair");
    info!("🔑 Identity: {}", payer.pubkey());

    // 4.2 Initialize Adapters (Infrastructure Layer)
    let pool_fetcher = Arc::new(pool_fetcher::PoolKeyFetcher::new(&bot_cfg.rpc_url));

    // Dynamic Executor Selection: Jito for Mainnet, Legacy for Devnet/Local
    let execution_port: Arc<dyn strategy::ports::ExecutionPort> = if bot_cfg.jito_url.is_empty() {
        info!("⚠️ Jito URL empty. Falling back to Legacy RPC Executor.");
        Arc::new(executor::legacy::LegacyExecutor::new(
            &bot_cfg.rpc_url,
            solana_sdk::signature::Keypair::from_bytes(&payer.to_bytes()).expect("Failed to clone keypair"),
            Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
        ))
    } else {
        match executor::jito::JitoExecutor::new(
            &bot_cfg.jito_url,
            &payer,
            &bot_cfg.rpc_url,
            Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>)
        ).await {
            Ok(jito) => Arc::new(jito),
            Err(e) => {
                warn!("❌ Jito initialization failed: {}. Falling back to Legacy.", e);
                Arc::new(executor::legacy::LegacyExecutor::new(
                    &bot_cfg.rpc_url,
                    solana_sdk::signature::Keypair::from_bytes(&payer.to_bytes()).expect("Failed to clone keypair"),
                    Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
                ))
            }
        }
    };

    // 4.3 Initialize Domain Services (Strategy Layer)
    let performance_tracker = Arc::new(strategy::analytics::performance::PerformanceTracker::new("logs/performance.log").await);
    let safety_checker = Arc::new(strategy::safety::token_validator::TokenSafetyChecker::new(&bot_cfg.rpc_url));

    let engine = Arc::new(StrategyEngine::new(
        Some(execution_port),
        None, // No simulation in prod
        None, // No AI model yet
        Some(Arc::clone(&performance_tracker)),
        Some(Arc::clone(&safety_checker)),
    ));

    let wallet_mgr = WalletManager::new(&bot_cfg.rpc_url);

    let metrics = Arc::new(metrics::BotMetrics::new());
    let risk_mgr = Arc::new(risk::RiskManager::new());
    
    // 4.3.5 Initialize Alerting
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
    let alert_mgr = Arc::new(alerts::AlertManager::new(bot_cfg.discord_webhook.clone(), telegram_config));

    // 4.3.6 Initialize Telemetry
    telemetry::init_metrics();
    tokio::spawn(telemetry::serve_metrics());
    
    // Start health monitor (1 hour status)
    tokio::spawn(alerts::monitor_health(Arc::clone(&alert_mgr), Arc::clone(&metrics)));

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
        alert_mgr,
    });

    // 4.5 Pre-flight Wallet Verification
    info!("🧪 Validating Wallet state for monitored tokens...");
    let mut unique_mints = std::collections::HashSet::new();
    for pool in config::MONITORED_POOLS {
        unique_mints.insert(pool.token_a);
        unique_mints.insert(pool.token_b);
    }

    for mint in &unique_mints {
        // Skip Native SOL as it doesn't need an ATA
        if *mint == mev_core::constants::SOL_MINT {
            continue;
        }

        if let Some(_ix) = context.wallet_mgr.ensure_ata_exists(&context.payer.pubkey(), &mint) {
            info!("📦 Auto-creating ATA for token: {}...", mint);
        }
    }

    // 4.6 Pre-flight Balance Checks (Gas & Capital)
    info!("💰 Checking balances...");
    match context.wallet_mgr.get_sol_balance(&context.payer.pubkey()) {
        Ok(balance) => {
            let sol = balance as f64 / 1e9;
            if balance < 50_000_000 { // 0.05 SOL
                warn!("⚠️ LOW SOL BALANCE: {:.4} SOL. Gas might run out during high activity.", sol);
            } else {
                info!("✅ SOL Balance: {:.4} SOL (Gas Safe)", sol);
            }
        }
        Err(e) => error!("❌ Failed to fetch SOL balance: {}", e),
    }

    info!("📊 --- STARTUP TOKEN INVENTORY ---");
    let mut inventory = std::collections::HashMap::new();
    unique_mints.remove(&mev_core::constants::SOL_MINT); // Already checked SOL
    for mint in unique_mints {
        match context.wallet_mgr.get_token_balance(&context.payer.pubkey(), &mint) {
            Ok(balance) => {
                let symbol = match mint {
                    mev_core::constants::USDC_MINT => "USDC",
                    mev_core::constants::JUP_MINT => "JUP ",
                    mev_core::constants::RAY_MINT => "RAY ",
                    mev_core::constants::BONK_MINT => "BONK",
                    mev_core::constants::WIF_MINT => "WIF ",
                    _ => "UNKN",
                };
                info!("   ├─ {}: {:.6} (raw: {})", symbol, balance as f64 / 1e6, balance); // Assuming 6 decimals for most (USDC/JUP etc)
                inventory.insert(symbol, balance);
            }
            Err(e) => error!("   ├─ Error fetching balance for {}: {}", mint, e),
        }
    }
    info!("   └─ Total: {} tokens tracked", inventory.len());
    info!("📊 -------------------------------");
    
    let (tx, mut rx) = mpsc::channel::<MarketUpdate>(1024);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    
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

    let ws_url = bot_cfg.ws_url.clone();
    let listener_tx = tx.clone();
    let listener_pools = pools_to_watch.clone();
    let _listener_handle = tokio::spawn(async move {
        loop {
            listener::start_listener(ws_url.clone(), listener_tx.clone(), listener_pools.clone()).await;
            warn!("🔗 WebSocket Listener exited. Reconnecting in 5s...");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    // 6. Shutdown Watcher (Best Practice: Coordinated Exit)
    let shutdown_tx_signal = shutdown_tx.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("🛑 Shutdown signal received (Ctrl+C). Cleaning up...");
        let _ = shutdown_tx_signal.send(()).await;
    });

    // 6.5. TUI Dashboard (Real-time Monitoring)
    let tui_state = Arc::new(std::sync::Mutex::new(tui::AppState::new()));
    let tui_state_clone = Arc::clone(&tui_state);
    
    // Spawn TUI in separate thread (not async)
    let _tui_handle = std::thread::spawn(move || {
        if let Err(e) = tui::TuiApp::new(tui_state_clone).run() {
            error!("TUI error: {}", e);
        }
    });

    info!("🔥 Engine IGNITION. Waiting for market events...");
    info!("📊 TUI Dashboard ACTIVE (press 'q' to quit)");

    // 🧪 FORCED_ARB_MOCK: Removed for Phase 2 Verification
    // High-fidelity simulation now uses real market data with simulation execution.

    // 7. The Core Loop
    loop {
        tokio::select! {
            // A. Process Market Events
            Some(event) = rx.recv() => {
                let domain_update = mev_core::PoolUpdate {
                    pool_address: event.pool_address,
                    program_id: event.program_id,
                    mint_a: event.coin_mint,
                    mint_b: event.pc_mint,
                    reserve_a: event.coin_reserve as u128,
                    reserve_b: event.pc_reserve as u128,
                    price_sqrt: event.price_sqrt,
                    liquidity: event.liquidity,
                    fee_bps: 30, 
                    timestamp: event.timestamp as u64,
                };

                // Record Market Data
                if let Some(r) = &recorder {
                    let r_clone = Arc::clone(r);
                    let update_clone = domain_update.clone();
                    tokio::spawn(async move {
                        r_clone.record(update_clone).await;
                    });
                }

                let ctx = Arc::clone(&context);
                let rec_inner = recorder.clone();
                tokio::spawn(async move {
                    // Update WebSocket status in telemetry
                    telemetry::WEBSOCKET_STATUS.set(1);

                    // 🛡️ Risk Check
                    if let Err(e) = ctx.risk_mgr.can_trade(ctx.config.default_trade_size_lamports) {
                        tracing::debug!("🚫 Trade blocked by RiskManager: {}", e);
                        return;
                    }

                    let start_time = std::time::Instant::now();
                    match ctx.engine.process_event(
                        domain_update, 
                        ctx.config.default_trade_size_lamports,
                        ctx.config.jito_tip_lamports,
                        ctx.config.jito_tip_percentage,
                        ctx.config.max_jito_tip_lamports,
                        ctx.config.max_slippage_bps,
                        ctx.config.volatility_sensitivity,
                        ctx.config.max_slippage_ceiling
                    ).await {
                        Ok(Some(opportunity)) => {
                            let duration = start_time.elapsed().as_millis() as f64;
                            telemetry::DETECTION_LATENCY.observe(duration);
                            telemetry::OPPORTUNITIES_TOTAL.inc();
                            telemetry::OPPORTUNITIES_PROFITABLE.inc();

                            // 📊 Metrics Tracking
                            ctx.metrics.log_opportunity(true, true);
                            
                            // 📉 Risk Recording
                            ctx.risk_mgr.record_trade(ctx.config.default_trade_size_lamports, opportunity.expected_profit_lamports as i64);

                            // telemetry::TRADES_EXECUTED.inc();
                            // telemetry::PROFIT_LAMPORTS.inc_by(opportunity.expected_profit_lamports as f64);

                            if let Some(r) = &rec_inner {
                                let _ = r.record_arbitrage(opportunity).await;
                            }
                        }
                        Ok(None) => {
                            telemetry::OPPORTUNITIES_TOTAL.inc();
                            // Opportunity was either not found or rejected by safety checks
                        }
                        Err(e) => {
                            telemetry::RPC_ERRORS.inc();
                            ctx.metrics.rpc_errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            error!("💥 Processing error: {}", e);
                        }
                    }
                });
            }
            
            _ = shutdown_rx.recv() => {
                info!("👋 Engine shutting down gracefully...");
                context.metrics.print_summary();
                info!("Goodbye!");
                break;
            }
            
            else => {
                info!("⚠️ Ingress channel closed. Exiting.");
                break;
            }
        }
    }
}
