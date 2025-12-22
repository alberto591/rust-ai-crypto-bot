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

    // 2. Infrastructure Infrastructure (Adapters)
    let executor = JitoExecutor::new(&config.jito_url, &payer, &config.rpc_url)
        .await
        .expect("CRITICAL: Failed to initialize Jito Executor");
    let execution_port = Arc::new(executor);
    
    info!("✅ Jito Block Engine connected (No-Auth Mode)");

    // 3. Domain Service (The Brain)
    // Here we wire the infrastructure (Executor) into the domain (StrategyEngine)
    let strategy_engine = Arc::new(StrategyEngine::new(
        Some(execution_port),
        None, // Simulation disabled for now
        None, // AI Model disabled (Heuristic mode)
    ));

    // 4. Pre-flight Wallet Verification
    info!("🧪 Validating Wallet state...");
    let wallet_mgr = wallet_manager::WalletManager::new(&config.rpc_url);
    let usdc_mint = devnet_keys::parse_pubkey(devnet_keys::USDC_MINT);
    
    if let Some(_ix) = wallet_mgr.ensure_ata_exists(&payer.pubkey(), &usdc_mint) {
        info!("📦 Auto-creating USDC ATA...");
        // In a real run, you'd execute this instruction.
    }

    // 5. Market Connectivity
    let (tx, mut rx) = mpsc::channel::<MarketUpdate>(1024);
    
    // Convert Comma-separated string to HashMap for the listener
    let mut pools_to_watch = HashMap::new();
    for addr in config.monitored_pool_addresses.split(',') {
        if !addr.trim().is_empty() {
            pools_to_watch.insert(addr.trim().to_string(), ("SOL".to_string(), "USDC".to_string()));
        }
    }

    let ws_url = config.ws_url.clone();
    tokio::spawn(async move {
        listener::start_listener(ws_url, tx, pools_to_watch).await;
    });

    info!("🔥 Engine IGNITION. Waiting for market events...");

    // 6. The Core Loop (Reactivity)
    while let Some(event) = rx.recv().await {
        // Map MarketUpdate -> PoolUpdate (Domain object transition)
        let domain_update = mev_core::PoolUpdate {
            pool_address: event.pool_address,
            program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
            mint_a: event.coin_mint,
            mint_b: event.pc_mint,
            reserve_a: event.coin_reserve as u128,
            reserve_b: event.pc_reserve as u128,
            fee_bps: 30, // Standard Raydium fee
            timestamp: event.timestamp as u64,
        };

        // Pass event into the Brain
        // Note: StrategyEngine now handles discovery + execution internally
        let engine_clone = Arc::clone(&strategy_engine);
        tokio::spawn(async move {
            if let Err(e) = engine_clone.process_event(domain_update).await {
                error!("Strategy Error: {}", e);
            }
        });
    }
}
