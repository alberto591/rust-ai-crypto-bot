pub mod raydium;
pub mod orca;
pub mod math;

use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoolUpdate {
    pub pool_address: Pubkey,
    pub program_id: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub reserve_a: u128,
    pub reserve_b: u128,
    pub fee_bps: u16,
    pub timestamp: u64,
}

/// A comprehensive market update signal
/// Carries both price (reserves) and topology (token mints) information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketUpdate {
    pub pool_address: Pubkey,
    pub coin_mint: Pubkey,  // Token A (e.g., SOL)
    pub pc_mint: Pubkey,    // Token B (e.g., USDC)
    pub coin_reserve: u64,
    pub pc_reserve: u64,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapStep {
    pub pool: Pubkey,
    pub program_id: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
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

pub mod constants {
    use solana_sdk::pubkey;
    use solana_sdk::pubkey::Pubkey;

    pub const JITO_TIP_PROGRAM: Pubkey = pubkey!("TipMessage111111111111111111111111111111111");
    
    pub const RAYDIUM_V4_PROGRAM: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
    pub const ORCA_WHIRLPOOL_PROGRAM: Pubkey = pubkey!("whirLbMiqkh6thXv7uBToywS9Bn1McGQ669YUsbAHQi");
}
