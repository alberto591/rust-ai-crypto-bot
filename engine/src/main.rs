use std::env;
use tokio::sync::mpsc;
use dotenvy::dotenv;
use solana_sdk::{
    signature::{read_keypair_file, Signer},
    pubkey::Pubkey,
};

// Internal Crates
// Note: We use mev_core, but the user code requested 'core'. 
// We alias it or use mev_core::MarketUpdate
use mev_core::MarketUpdate;
use strategy::{graph::MarketGraph, arb::ArbFinder};
use executor::legacy::LegacyExecutor;

// Import our Devnet Constants
mod devnet_keys;
mod listener;

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("🚀 Starting HFT Engine [INTEGRATED MODE]...");
    println!("=========================================\n");

    // 1. Setup Configuration
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let key_path = env::var("KEYPAIR_PATH").unwrap_or_else(|_| 
        format!("{}/.config/solana/id.json", env::var("HOME").unwrap())
    );
    
    // Create keypair if it doesn't exist (for seamless testing)
    if !std::path::Path::new(&key_path).exists() {
        println!("⚠️  Keypair not found at {}, creating temporary one...", key_path);
        // We'd normally use the generate_keypair example logic here, but for now we expect it to exist
        // or we fail gracefully.
    }

    let payer = read_keypair_file(&key_path).unwrap_or_else(|_| {
        // Fallback for CI/Testing without keypair
        solana_sdk::signature::Keypair::new() 
    });
    
    let payer_pubkey = payer.pubkey();
    println!("🔑 Trading Identity: {}", payer_pubkey);

    // 2. Initialize Components
    let mut graph = MarketGraph::new();
    let _executor = LegacyExecutor::new(&rpc_url); // Kept for future use

    // 3. Create the Internal Nerve System (Channel)
    // The Listener writes to 'tx', Main Loop reads from 'rx'
    let (tx, mut rx) = mpsc::channel::<MarketUpdate>(1000);

    // 4. Spawn the Listener (The Eyes)
    // For this DRY RUN, we will simulate the Listener to prove the Graph works
    // without waiting for a real blockchain block update.
    tokio::spawn(async move {
        println!("👀 Listener Active: Monitoring Devnet Pools...");
        loop {
            // SIMULATION: Create a fake market movement every 2 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            println!("... Simulating Market Updates (SOL -> USDC -> RAY -> SOL) ...");

            // 1. SOL -> USDC (Price: 1 SOL = 100 USDC)
            let update_1 = MarketUpdate {
                pool_address: devnet_keys::parse_pubkey(devnet_keys::SOL_USDC_AMM_ID),
                coin_mint: devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT),
                pc_mint: devnet_keys::parse_pubkey(devnet_keys::USDC_MINT),
                coin_reserve: 1_000_000_000,      // 1 SOL
                pc_reserve: 100_000_000,          // 100 USDC
                timestamp: 0,
            };
            let _ = tx.send(update_1).await;

            // 2. USDC -> RAY (Price: 1 USDC = 1 RAY) - Cheap RAY!
            let update_2 = MarketUpdate {
                pool_address: devnet_keys::parse_pubkey(devnet_keys::USDC_RAY_AMM_ID),
                coin_mint: devnet_keys::parse_pubkey(devnet_keys::USDC_MINT),
                pc_mint: devnet_keys::parse_pubkey(devnet_keys::RAY_MINT),
                coin_reserve: 1_000_000,          // 1 USDC
                pc_reserve: 1_000_000,            // 1 RAY
                timestamp: 0,
            };
            let _ = tx.send(update_2).await;

            // 3. RAY -> SOL (Price: 1 RAY = 0.015 SOL) - Expensive RAY!
            // Arbitrage: Buy RAY with USDC, sell RAY for more SOL
            let update_3 = MarketUpdate {
                pool_address: devnet_keys::parse_pubkey(devnet_keys::RAY_SOL_AMM_ID),
                coin_mint: devnet_keys::parse_pubkey(devnet_keys::RAY_MINT),
                pc_mint: devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT),
                coin_reserve: 1_000_000,          // 1 RAY
                pc_reserve: 15_000_000,           // 0.015 SOL
                timestamp: 0,
            };
            if let Err(_) = tx.send(update_3).await {
                break;
            }
        }
    });

    // 5. The Main Event Loop (The Brain)
    println!("🧠 Brain Active: Waiting for signals...");
    while let Some(event) = rx.recv().await {
        
        // A. Update the Graph (Memory)
        graph.update_edge(
            event.coin_mint, 
            event.pc_mint, 
            event.pool_address, 
            event.coin_reserve, 
            event.pc_reserve
        );
        // Also update the reverse edge (Bid/Ask)
        graph.update_edge(
            event.pc_mint, 
            event.coin_mint, 
            event.pool_address, 
            event.pc_reserve, 
            event.coin_reserve
        );

        // B. Run Strategy (Search for 0.01 SOL -> ??? -> 0.01+ SOL)
        // We look for a cycle starting with SOL
        let amount_in = 10_000_000; // 0.01 SOL
        let wsol_mint = devnet_keys::parse_pubkey(devnet_keys::WSOL_MINT);

        // NOTE: Since we only have 1 pool in the graph (SOL/USDC), a 3-hop cycle is impossible.
        // The ArbFinder needs at least 3 pools to work (SOL->USDC->RAY->SOL).
        // However, we check anyway to prove the function runs without crashing.
        if let Some(path) = ArbFinder::find_best_cycle(&graph, wsol_mint, amount_in) {
            println!("🚨 ARBITRAGE FOUND! Expected Profit: {}", path.expected_profit);
            
            // C. Execute (The Hands)
            for hop in path.hops {
                println!("   -> Swap on Pool: {}", hop.pool_address);
            }
        } else {
            // Standard output to show it's thinking
            println!("Checking... Graph Size: {} Tokens. No Arb found (Need more pools).", graph.adj.len());
        }
    }
}
