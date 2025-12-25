pub mod raydium;
pub mod orca;
pub mod meteora;
pub mod math;

use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoolUpdate {
    pub pool_address: Pubkey,
    pub program_id: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub reserve_a: u128,      // Used for CPMM
    pub reserve_b: u128,      // Used for CPMM
    pub price_sqrt: Option<u128>, // Used for CLMM (Orca) - X64
    pub liquidity: Option<u128>,  // Used for CLMM (Orca)
    pub fee_bps: u16,
    pub timestamp: u64,
}

/// A comprehensive market update signal
/// Carries both price (reserves) and topology (token mints) information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketUpdate {
    pub pool_address: Pubkey,
    pub program_id: Pubkey, // Added for DEX identification
    pub coin_mint: Pubkey,  // Token A (e.g., SOL)
    pub pc_mint: Pubkey,    // Token B (e.g., USDC)
    pub coin_reserve: u64,
    pub pc_reserve: u64,
    pub price_sqrt: Option<u128>, // CLMM support
    pub liquidity: Option<u128>,  // CLMM support
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapStep {
    pub pool: Pubkey,
    pub program_id: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub expected_output: u64, // Added to track amount through multi-hop
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArbitrageOpportunity {
    pub steps: Vec<SwapStep>,
    pub expected_profit_lamports: u64,
    pub input_amount: u64,
    pub total_fees_bps: u16,
    pub max_price_impact_bps: u16,
    pub min_liquidity: u128,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum DexType {
    Raydium,
    Orca,
}

pub mod constants {
    use solana_sdk::pubkey;
    use solana_sdk::pubkey::Pubkey;

    pub const JITO_TIP_PROGRAM: Pubkey = pubkey!("TipMessage111111111111111111111111111111111");
    
    pub const RAYDIUM_V4_PROGRAM: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
    pub const ORCA_WHIRLPOOL_PROGRAM: Pubkey = pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
    pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    // Token Mints
    pub const SOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
    pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    pub const JUP_MINT: Pubkey = pubkey!("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN");
    pub const RAY_MINT: Pubkey = pubkey!("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R");
    pub const BONK_MINT: Pubkey = pubkey!("DezXAZ8z7Pnrn9kvJdVyX6VKDrKA6diBr2PzgQTqbMZ6");
    pub const WIF_MINT: Pubkey = pubkey!("EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm");
}
