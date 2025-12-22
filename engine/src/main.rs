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

mod listener;
mod pool_fetcher;
mod devnet_keys;
mod wallet_manager;
mod config;

use crate::wallet_manager::WalletManager;

/// Global Application Context
/// Shared, read-only resources wired together at startup
pub struct AppContext {
    pub config: config::BotConfig,
    pub payer: solana_sdk::signature::Keypair,
    pub engine: Arc<StrategyEngine>,
    pub wallet_mgr: WalletManager,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    // Initialize tracing (Logging)
    tracing_subscriber::fmt::init();
    
    info!("🚀 HFT Engine Bootstrapping [Composition Root]...");

    // 1. Unified Configuration Layer
    let config = config::BotConfig::new().expect("CRITICAL: Failed to load config. Check .env");
    info!("✅ Config Loaded: RPC={}, Jito={}", config.rpc_url, config.jito_url);
    
    let key_path = if config.keypair_path.is_empty() {
        format!("{}/.config/solana/id.json", env::var("HOME").unwrap())
    } else {
        config.keypair_path.clone()
    };
    
    let payer = read_keypair_file(&key_path).expect("CRITICAL: Failed to load keypair");
    info!("🔑 Identity: {}", payer.pubkey());

    // 2. Infrastructure (Adapters)
    let executor = JitoExecutor::new(&config.jito_url, &payer, &config.rpc_url)
        .await
        .expect("CRITICAL: Failed to initialize Jito Executor");
    let execution_port = Arc::new(executor);
    
    info!("✅ Jito Block Engine connected (No-Auth Mode)");

    // 3. Domain Service (The Brain)
    let strategy_engine = Arc::new(StrategyEngine::new(
        Some(execution_port),
        None, // Simulation disabled
        None, // AI Model disabled
    ));

    // 4. Composition: Wiring AppContext
    let context = Arc::new(AppContext {
        config: config.clone(),
        payer,
        engine: strategy_engine,
        wallet_mgr: WalletManager::new(&config.rpc_url),
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
    let _listener_handle = tokio::spawn(async move {
        listener::start_listener(ws_url, tx, pools_to_watch).await;
    });

    // 6. Shutdown Watcher (Best Practice: Coordinated Exit)
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("🛑 Shutdown signal received (Ctrl+C). Cleaning up...");
        let _ = shutdown_tx.send(()).await;
    });

    info!("🔥 Engine IGNITION. Waiting for market events...");

    // 7. The Core Loop
    loop {
        tokio::select! {
            // A. Process Market Events
            Some(event) = rx.recv() => {
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
                tokio::spawn(async move {
                    if let Err(e) = ctx.engine.process_event(domain_update).await {
                        error!("Strategy Error: {}", e);
                    }
                });
            }
            
            // B. Coordinated Shutdown
            _ = shutdown_rx.recv() => {
                info!("👋 Engine shutting down gracefully. Goodbye!");
                break;
            }
            
            // C. Channel closed
            else => {
                info!("⚠️ Ingress channel closed. Exiting.");
                break;
            }
        }
    }
}
