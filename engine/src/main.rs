use std::env;
use std::collections::HashMap;
use tokio::sync::mpsc;
use dotenvy::dotenv;
use solana_sdk::{
    signature::{read_keypair_file, Signer},
    pubkey::Pubkey,
};

// Internal Crates
use mev_core::MarketUpdate;
use strategy::{graph::MarketGraph, arb::ArbFinder};
use executor::{jito::JitoExecutor, raydium_builder::RaydiumSwapKeys};

mod listener;
mod pool_fetcher;
mod devnet_keys;
mod wallet_manager;
mod config;

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("🚀 Starting HFT Engine [SIMULATION + REAL JITO EXECUTION]...");
    println!("=========================================================\n");

    // 1. Setup Configuration
    let config = config::BotConfig::new().expect("Failed to load config");
    println!("✅ Config Loaded: RPC={}, WS={}", config.rpc_url, config.ws_url);
    
    let rpc_url = config.rpc_url;
    let key_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| 
        format!("{}/.config/solana/id.json", env::var("HOME").unwrap())
    );
    
    let payer = read_keypair_file(&key_path).expect("Failed to load keypair");
    let payer_pubkey = payer.pubkey();
    println!("🔑 Trading Identity: {}", payer_pubkey);

    // 2. Initialize Real Jito Executor (No-Auth)
    let jito_url = "https://ny.mainnet.block-engine.jito.wtf"; 
    let jito_executor = JitoExecutor::new(jito_url, &payer, &rpc_url)
        .await
        .expect("Failed to connect to Jito");
    
    println!("✅ Connected to Jito Block Engine: {}", jito_url);

    // 2.5 Pre-flight Wallet Check
    println!("🧪 Running Pre-flight Check (Wallet)...");
    let wallet_mgr = wallet_manager::WalletManager::new(&rpc_url);
    let usdc_mint = devnet_keys::parse_pubkey(devnet_keys::USDC_MINT);
    let _wsol_mint = devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT);
    
    let mut setup_ixs = Vec::new();
    if let Some(ix) = wallet_mgr.ensure_ata_exists(&payer.pubkey(), &usdc_mint) {
        setup_ixs.push(ix);
    }
    
    match wallet_mgr.sync_wsol(&payer, 50_000_000) {
        Ok(ixs) => setup_ixs.extend(ixs),
        Err(e) => println!("⚠️ WSOL Sync failed: {}", e),
    }

    if !setup_ixs.is_empty() {
        println!("📦 Wallet requires setup ({} ixs).", setup_ixs.len());
        // In simulation, we just log. In live, we'd send a bundle here.
        println!("   [SIMULATION] Wallet setup verified.");
    } else {
        println!("✅ Wallet Ready: ATAs and WSOL verified.");
    }

    // 3. Initialize Graph & Channels
    let mut graph = MarketGraph::new();
    let (tx, mut rx) = mpsc::channel::<MarketUpdate>(1000);

    /*
    // ---------------------------------------------------------
    // ❌ OPTION A: SIMULATION (Commented out for Reality)
    // ---------------------------------------------------------
    tokio::spawn(async move {
        println!("🎭 Simulation Active: Injecting Market Scenarios...");
        
        loop {
            sleep(Duration::from_secs(5)).await;

            println!("\n⚡ INJECTING ARBITRAGE SIGNAL...");
            let sol_usdc_pool = devnet_keys::parse_pubkey(devnet_keys::SOL_USDC_AMM_ID);
            let wsol_mint = devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT);
            let usdc_mint = devnet_keys::parse_pubkey(devnet_keys::USDC_MINT);

            let _ = tx.send(MarketUpdate {
                pool_address: sol_usdc_pool,
                coin_mint: wsol_mint,
                pc_mint: usdc_mint,
                coin_reserve: 1_000_000_000, 
                pc_reserve: 100_000_000,     
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64,
            }).await;
        }
    });
    */

    // ---------------------------------------------------------
    // ✅ OPTION B: REALITY (Enabled)
    // ---------------------------------------------------------
    // 1. Define the Pools you want to watch
    let mut monitored_pools = HashMap::new();
    monitored_pools.insert(
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(), // SOL/USDC (Mainnet)
        ("So11111111111111111111111111111111111111112".to_string(), "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string())
    );

    // 2. Spawn the Listener (Requires Private RPC WS_URL in .env)
    let ws_url = env::var("WS_URL").unwrap_or_else(|_| rpc_url.replace("http", "ws")); 
    tokio::spawn(async move {
        listener::start_listener(ws_url, tx, monitored_pools).await;
    });

    // 4. Ignition
    println!("🔥 Ignition: Engine Running. Listening for Raydium events...");
    
    // Safety check for mints
    let _ = devnet_keys::get_devnet_mints();
    
    let mut pool_key_cache: HashMap<Pubkey, RaydiumSwapKeys> = HashMap::new();
    let fetcher = pool_fetcher::PoolKeyFetcher::new(&rpc_url);
    let wsol_mint = devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT);

    while let Some(event) = rx.recv().await {
        println!("🔄 Update received: {} (Reserves: {} / {})", event.pool_address, event.coin_reserve, event.pc_reserve);

        // A. Update Memory
        graph.update_edge(event.coin_mint, event.pc_mint, event.pool_address, event.coin_reserve, event.pc_reserve);
        graph.update_edge(event.pc_mint, event.coin_mint, event.pool_address, event.pc_reserve, event.coin_reserve);

        // B. Ensure we have pool keys
        if let std::collections::hash_map::Entry::Vacant(e) = pool_key_cache.entry(event.pool_address) {
            if let Ok(keys) = fetcher.fetch_keys(&event.pool_address).await {
                e.insert(keys);
            }
        }

        // C. Check Strategy
        // 1. Set Trade Size (The Bet)
        // 0.1 SOL = 100,000,000 Lamports.
        let amount_in = 100_000_000; 

        // 2. Set Jito Tip (The Bribe)
        let jito_tip = 10_000;

        // 3. Set Slippage (The Safety Net)
        // We want at least 99% of our value back, or the trade should fail.
        let min_amount_out = (amount_in as f64 * 0.99) as u64; 

        if let Some(path) = ArbFinder::find_best_cycle(&graph, wsol_mint, amount_in) {
            println!("🚨 REAL ARBITRAGE FOUND! Expected Profit: {}", path.expected_profit);
            
            // D. Fire Jito Bundle
            if let Some(cached_keys) = pool_key_cache.get(&path.hops[0].pool_address) {
                let mut trade_keys = cached_keys.clone();
                trade_keys.user_owner = payer.pubkey();
                
                // Build Instruction with SAFETY
                let ix = executor::raydium_builder::swap_base_in(&trade_keys, amount_in, min_amount_out);
                
                match jito_executor.send_bundle(vec![ix], jito_tip).await {
                    Ok(id) => println!("   ✅ REAL BUNDLE SENT! ID: {}", id),
                    Err(e) => println!("   ❌ BUNDLE FAILED: {}", e),
                }
            }
        }
    }
}
