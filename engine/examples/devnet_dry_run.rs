/// Devnet Dry Run Test
/// 
/// This is a minimal test to verify the Raydium instruction builder works
/// by executing a real swap on Solana Devnet.

use executor::{legacy::LegacyExecutor, raydium_builder::swap_base_in};
use mev_core::raydium::RaydiumSwapKeys;
use solana_sdk::{
    signature::{Keypair, read_keypair_file},
    signer::Signer,
    pubkey::Pubkey,
};
use std::str::FromStr;
use spl_token;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Devnet Dry Run: Testing Raydium Swap Builder");
    println!("================================================\n");

    // 1. Load wallet from keypair file or create new one for testing
    // For devnet testing, you can create a throwaway keypair
    let payer = Keypair::new();
    println!("💰 Wallet: {}", payer.pubkey());
    println!("⚠️  Make sure this wallet has SOL on devnet!");
    println!("   Airdrop: solana airdrop 2 {} --url devnet\n", payer.pubkey());

    // 2. Set up Legacy Executor (Devnet RPC)
    let executor = LegacyExecutor::new("https://api.devnet.solana.com");
    println!("🌐 Connected to Devnet RPC\n");

    // 3. Hardcoded Raydium SOL-USDC Pool on Devnet
    // NOTE: These addresses are examples - you'll need to get real Devnet pool addresses
    // from Raydium's devnet deployment or their API
    let pool_keys = RaydiumSwapKeys {
        // AMM Pool (main account)
        amm_id: Pubkey::from_str("DEVNET_AMM_POOL_ADDRESS_HERE")
            .expect("Invalid AMM ID"),
        
        // AMM Authority (PDA derived from AMM ID)
        amm_authority: Pubkey::from_str("DEVNET_AMM_AUTHORITY_HERE")
            .expect("Invalid AMM Authority"),
        
        // AMM Accounts
        amm_open_orders: Pubkey::from_str("DEVNET_OPEN_ORDERS_HERE")
            .expect("Invalid Open Orders"),
        amm_target_orders: Pubkey::from_str("DEVNET_TARGET_ORDERS_HERE")
            .expect("Invalid Target Orders"),
        
        // Pool Vaults
        amm_coin_vault: Pubkey::from_str("DEVNET_COIN_VAULT_HERE")
            .expect("Invalid Coin Vault"),
        amm_pc_vault: Pubkey::from_str("DEVNET_PC_VAULT_HERE")
            .expect("Invalid PC Vault"),
        
        // Serum DEX Info
        serum_program_id: Pubkey::from_str("DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY")
            .expect("Invalid Serum Program"),
        serum_market: Pubkey::from_str("DEVNET_SERUM_MARKET_HERE")
            .expect("Invalid Serum Market"),
        serum_bids: Pubkey::from_str("DEVNET_BIDS_HERE")
            .expect("Invalid Bids"),
        serum_asks: Pubkey::from_str("DEVNET_ASKS_HERE")
            .expect("Invalid Asks"),
        serum_event_queue: Pubkey::from_str("DEVNET_EVENT_QUEUE_HERE")
            .expect("Invalid Event Queue"),
        serum_coin_vault: Pubkey::from_str("DEVNET_SERUM_COIN_VAULT_HERE")
            .expect("Invalid Serum Coin Vault"),
        serum_pc_vault: Pubkey::from_str("DEVNET_SERUM_PC_VAULT_HERE")
            .expect("Invalid Serum PC Vault"),
        serum_vault_signer: Pubkey::from_str("DEVNET_VAULT_SIGNER_HERE")
            .expect("Invalid Vault Signer"),
        
        // User Token Accounts (you'll need to create these)
        user_source_token_account: Pubkey::from_str("YOUR_SOL_TOKEN_ACCOUNT")
            .expect("Invalid source token account"),
        user_dest_token_account: Pubkey::from_str("YOUR_USDC_TOKEN_ACCOUNT")
            .expect("Invalid dest token account"),
        
        // User
        user_owner: payer.pubkey(),
        
        // SPL Token Program
        token_program: spl_token::ID,
    };

    println!("📦 Pool Configuration:");
    println!("   AMM: {}", pool_keys.amm_id);
    println!("   Source: {}", pool_keys.user_source_token_account);
    println!("   Dest: {}", pool_keys.user_dest_token_account);
    println!();

    // 4. Build Swap Instruction
    let amount_in = 100_000_000; // 0.1 SOL
    let min_amount_out = 0; // No slippage protection for test (set properly in production!)
    
    println!("💱 Building Swap Instruction:");
    println!("   Amount In: {} lamports ({} SOL)", amount_in, amount_in as f64 / 1e9);
    println!("   Min Out: {} (no slippage protection)", min_amount_out);
    println!();

    let swap_ix = swap_base_in(&pool_keys, amount_in, min_amount_out);

    // 5. Execute Transaction
    println!("🚀 Executing transaction...");
    match executor.execute_standard_tx(&payer, &[swap_ix]) {
        Ok(signature) => {
            println!("\n✅ SUCCESS!");
            println!("📝 Transaction Signature: {}", signature);
            println!("🔍 View on Solscan: https://solscan.io/tx/{}?cluster=devnet", signature);
        }
        Err(e) => {
            println!("\n❌ FAILED!");
            println!("Error: {}", e);
            println!("\nCommon issues:");
            println!("  - Insufficient SOL for fees");
            println!("  - Token accounts don't exist");
            println!("  - Pool addresses are incorrect");
            println!("  - Network connectivity issues");
        }
    }

    Ok(())
}
