pub mod raydium;
pub mod orca;
pub mod meteora;
pub mod math;

use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;

use smallvec::SmallVec;

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
    pub steps: SmallVec<[SwapStep; 8]>,
    pub expected_profit_lamports: u64,
    pub input_amount: u64,
    pub total_fees_bps: u16,
    pub max_price_impact_bps: u16,
    pub min_liquidity: u128,
    pub timestamp: u64,
    pub is_dna_match: bool,    // Added for Phase 11 Telemetry
    pub is_elite_match: bool,  // Added for Phase 11 Telemetry
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
    pub const USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
    pub const POPCAT_MINT: Pubkey = pubkey!("7GCihg6ndP4RfSfAeKSV1bGBzBRQ6TsmNgzLfdPNs7Ha");
    pub const JTO_MINT: Pubkey = pubkey!("jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL");
    pub const PENGU_MINT: Pubkey = pubkey!("2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv");
    pub const DRIFT_MINT: Pubkey = pubkey!("DriFtPZW76QCJj8fT4PkP8An3qcwc7pUnL9f1KxcyxBc");
    pub const BODEN_MINT: Pubkey = pubkey!("3psH1Mj1f7yUfaD5gh6Zj7epE8hhrMkMETgv5TshQA4o");

    // Discovery Constants
    pub const PUMP_FUN_PROGRAM: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
    pub const RAYDIUM_AMM_LOG_TRIGGER: &str = "initialize2";
    pub const PUMP_FUN_LOG_TRIGGER: &str = "Create";
}

/// A "Success Story" or "Library Entry" represents the DNA of a profitable trade
/// Used for market intelligence and strategy optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessStory {
    pub strategy_id: String,         // e.g., "momentum_sniper_v1"
    pub token_address: String,       // Solana token mint
    pub market_context: String,      // "Q4_Memecoin_Season", "Bear_Market_Recovery"
    pub lesson: String,              // Human-readable takeaway
    pub timestamp: u64,              // Unix timestamp

    // Entry Triggers (what convinced us to enter)
    pub liquidity_min: u64,
    pub has_twitter: bool,
    pub mint_renounced: bool,
    pub initial_market_cap: u64,

    // Performance Stats
    pub peak_roi: f64,               // Max observed ROI (%)
    pub time_to_peak_secs: u64,      // How long until peak
    pub drawdown: f64,               // Max loss from peak (%)

    pub is_false_positive: bool,     // True if this was a failed trade

    // Enhanced Context (Phase 6 - for ML training)
    pub holder_count_at_peak: Option<u64>,  // Number of holders at peak
    pub market_volatility: Option<f64>,     // Broader market volatility (BTC/ETH)
    pub launch_hour_utc: Option<u8>,        // Hour of day token launched (0-23)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessAnalysis {
    pub average_peak_roi: f64,
    pub median_time_to_peak: f64,
    pub total_successful_launches: usize,
    pub strategy_effectiveness: f64,  // % of non-false-positive trades
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDNA {
    pub initial_liquidity: u64,
    pub initial_market_cap: u64,
    pub launch_hour_utc: u8,
    pub has_twitter: bool,
    pub mint_renounced: bool,
    pub market_volatility: f64,
}
