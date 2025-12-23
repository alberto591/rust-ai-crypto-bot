use std::env;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use dotenvy::dotenv;
use solana_sdk::signature::{read_keypair_file, Signer};
use tracing::{info, error};

// Internal Crates
use mev_core::MarketUpdate;
use strategy::{StrategyEngine};
use executor::jito::JitoExecutor;
use executor::legacy::LegacyExecutor;

mod listener;
mod pool_fetcher;
mod devnet_keys;
mod wallet_manager;
mod config;
mod tui;

use crate::wallet_manager::WalletManager;

/// Global Application Context
/// Shared, read-only resources wired together at startup
pub struct AppContext {
    pub config: config::BotConfig,
    pub payer: solana_sdk::signature::Keypair,
    pub engine: Arc<StrategyEngine>,
    pub wallet_mgr: WalletManager,
    pub performance_tracker: Arc<strategy::analytics::performance::PerformanceTracker>,
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
    let config = match config::BotConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("❌ CRITICAL: Failed to load config: {}", e);
            std::process::exit(1);
        }
    };
    
    // 4. Startup Validation (Fail Fast)
    if let Err(e) = config.validate() {
        error!("❌ Configuration Validation Failed: {}", e);
        std::process::exit(1);
    }
    
    info!("✅ Config Loaded & Validated: RPC={}, Jito={}", config.rpc_url, config.jito_url);
    
    let key_path = if config.keypair_path.is_empty() {
        format!("{}/.config/solana/id.json", env::var("HOME").unwrap())
    } else {
        config.keypair_path.clone()
    };
    
    let payer = read_keypair_file(&key_path).expect("Failed to read keypair");
    info!("🔑 Identity: {}", payer.pubkey());

    // 4.2 Initialize Adapters (Infrastructure Layer)
    let pool_fetcher = Arc::new(pool_fetcher::PoolKeyFetcher::new(&config.rpc_url));

    // Dynamic Executor Selection: Jito for Mainnet, Legacy for Devnet/Local
    let execution_port: Arc<dyn strategy::ports::ExecutionPort> = if config.jito_url.is_empty() {
        info!("⚠️ Jito URL empty. Falling back to Legacy RPC Executor.");
        Arc::new(executor::legacy::LegacyExecutor::new(
            &config.rpc_url,
            solana_sdk::signature::Keypair::from_bytes(&payer.to_bytes()).expect("Failed to clone keypair"),
            Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
        ))
    } else {
        match JitoExecutor::new(
            &config.jito_url,
            &payer,
            &config.rpc_url,
            Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>)
        ).await {
            Ok(jito) => Arc::new(jito),
            Err(e) => {
                error!("❌ Jito initialization failed: {}. Falling back to Legacy.", e);
                Arc::new(executor::legacy::LegacyExecutor::new(
                    &config.rpc_url,
                    solana_sdk::signature::Keypair::from_bytes(&payer.to_bytes()).expect("Failed to clone keypair"),
                    Some(Arc::clone(&pool_fetcher) as Arc<dyn strategy::ports::PoolKeyProvider>),
                ))
            }
        }
    };

    // 4.3 Initialize Domain Services (Strategy Layer)
    let performance_tracker = Arc::new(strategy::analytics::performance::PerformanceTracker::new("logs/performance.log").await);
    let safety_checker = Arc::new(strategy::safety::token_validator::TokenSafetyChecker::new(&config.rpc_url));

    let engine = Arc::new(StrategyEngine::new(
        Some(execution_port),
        None, // No simulation in prod
        None, // No AI model yet
        Some(Arc::clone(&performance_tracker)),
        Some(Arc::clone(&safety_checker)),
    ));

    let wallet_mgr = WalletManager::new(&config.rpc_url);

    // 4.4 Assemble Context (Composition Root)
    let context = Arc::new(AppContext {
        config: config.clone(),
        payer,
        engine,
        wallet_mgr,
        performance_tracker,
    });

    // 4.5 Pre-flight Wallet Verification
    info!("🧪 Validating Wallet state...");
    let usdc_mint = devnet_keys::parse_pubkey(devnet_keys::USDC_MINT);
    if let Some(_ix) = context.wallet_mgr.ensure_ata_exists(&context.payer.pubkey(), &usdc_mint) {
        info!("📦 Auto-creating USDC ATA...");
    }
    
    let (tx, mut rx) = mpsc::channel::<MarketUpdate>(1024);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    
    let mut pools_to_watch = HashMap::new();
    for addr in config.monitored_pool_addresses.split(',') {
        if !addr.trim().is_empty() {
            pools_to_watch.insert(addr.trim().to_string(), ("SOL".to_string(), "USDC".to_string()));
        }
    }

    let ws_url = config.ws_url.clone();
    let listener_tx = tx.clone();
    let listener_pools = pools_to_watch.clone();
    let _listener_handle = tokio::spawn(async move {
        listener::start_listener(ws_url, listener_tx, listener_pools).await;
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

    // 🧪 FORCED_ARB_MOCK: Inject a profitable cycle for verification
    if std::env::var("FORCED_ARB_MOCK").unwrap_or_default() == "true" {
        info!("🧪 FORCED_ARB_MOCK active. Injecting profitable cycle in 5s...");
        let mock_tx = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            let sol = devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT);
            let usdc = devnet_keys::parse_pubkey(devnet_keys::USDC_MINT);
            let usdt = solana_sdk::pubkey::Pubkey::new_unique(); // Placeholder for triangular

            let pool_id = devnet_keys::parse_pubkey(devnet_keys::SOL_USDC_AMM_ID);

            // 1. SOL -> USDC (Mocking using the real pool ID)
            let _ = mock_tx.send(mev_core::MarketUpdate {
                pool_address: pool_id,
                coin_mint: sol,
                pc_mint: usdc,
                coin_reserve: 1_000_000_000_000, 
                pc_reserve: 200_000_000_000,
                timestamp: 0,
            }).await;

            // 2. USDC -> SOL (Reversing back for a 2-hop profitable mock)
            let _ = mock_tx.send(mev_core::MarketUpdate {
                pool_address: pool_id,
                coin_mint: usdc,
                pc_mint: sol,
                coin_reserve: 180_000_000_000, // Price discrepancy for mock profit
                pc_reserve: 1_200_000_000_000,
                timestamp: 0,
            }).await;
        });
    }

    // 7. The Core Loop
    loop {
        tokio::select! {
            // A. Process Market Events
            Some(event) = rx.recv() => {
                // Update TUI pool count
                if let Ok(mut state) = tui_state.lock() {
                    state.pool_count += 1;
                }
                
                let domain_update = mev_core::PoolUpdate {
                    pool_address: event.pool_address,
                    program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
                    mint_a: event.coin_mint,
                    mint_b: event.pc_mint,
                    reserve_a: event.coin_reserve as u128,
                    reserve_b: event.pc_reserve as u128,
                    fee_bps: 30,
                    timestamp: event.timestamp as u64,
                };

                let ctx = Arc::clone(&context);
                let tui_clone = Arc::clone(&tui_state);
                tokio::spawn(async move {
                    match ctx.engine.process_event(
                        domain_update, 
                        ctx.config.default_trade_size_lamports,
                        ctx.config.jito_tip_lamports,
                        ctx.config.max_slippage_bps
                    ).await {
                        Ok(Some(opportunity)) => {
                            if let Ok(mut state) = tui_clone.lock() {
                                state.total_simulated_pnl += opportunity.expected_profit_lamports;
                                state.recent_opportunities.push(opportunity.clone());
                                if state.recent_opportunities.len() > 10 {
                                    state.recent_opportunities.remove(0);
                                }
                                state.recent_logs.push(format!("✅ Opportunity: {} lamports", opportunity.expected_profit_lamports));
                                if state.recent_logs.len() > 20 {
                                    state.recent_logs.remove(0);
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(e) => {
                            error!("Strategy Error: {}", e);
                            if let Ok(mut state) = tui_clone.lock() {
                                state.recent_logs.push(format!("❌ Error: {}", e));
                                if state.recent_logs.len() > 20 {
                                    state.recent_logs.remove(0);
                                }
                            }
                        }
                    }
                });
            }
            
            _ = shutdown_rx.recv() => {
                info!("👋 Engine shutting down gracefully. Goodbye!");
                break;
            }
            
            else => {
                info!("⚠️ Ingress channel closed. Exiting.");
                break;
            }
        }
    }
}
