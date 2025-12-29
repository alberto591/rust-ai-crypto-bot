use std::env;
// use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::pubkey;
use mev_core::{DexType, FeeStrategy};
use mev_core::constants::*;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub dex: DexType,
}

pub const MONITORED_POOLS: &[PoolConfig] = &[
    // --- 🏮 RAYDIUM CORE (High Volume) ---
    PoolConfig { address: pubkey!("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"), token_a: SOL_MINT, token_b: USDC_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("7XJ7EtcbqB3SDeAnpKMf8B2Y4V5SgSjtup3jV5vAnN4"), token_a: SOL_MINT, token_b: USDT_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("AVs9TA4nWDzfPJE9gGVNJMVhcQy3V9PGazuz33BfG2RA"), token_a: RAY_MINT, token_b: SOL_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("3Fy7Py96FXtUtPYs8UPXAYqVjATbcKvN5FJGnSUNckSk"), token_a: SOL_MINT, token_b: JUP_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("Hm8DswhFx7VKXRJcdZ8bEMZvpVfQZNP6GFtHPvqgmLmD"), token_a: BONK_MINT, token_b: SOL_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("EP2ib6dYdEeqD8MfE2ezHCxX3kP3K2eLKkirfPm5eyMx"), token_a: WIF_MINT, token_b: SOL_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("8QaXeHBrShJTdtN1rWCccBxpSVvKksQ2PCu5nufb2zbk"), token_a: POPCAT_MINT, token_b: SOL_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("364rP7Y53NisS9mK4FpXwYF618D3b5YF488888888888"), token_a: JTO_MINT, token_b: SOL_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv"), token_a: PENGU_MINT, token_b: SOL_MINT, dex: DexType::Raydium },

    // --- 🌊 ORCA WHIRLPOOLS (Cross-DEX Targets) ---
    PoolConfig { address: pubkey!("HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"), token_a: SOL_MINT, token_b: USDC_MINT, dex: DexType::Orca },
    PoolConfig { address: pubkey!("5zpyutJu9ee6jFymDGoK7F6S5Kczqtc9FomP3ueKuyA9"), token_a: BONK_MINT, token_b: SOL_MINT, dex: DexType::Orca },
    PoolConfig { address: pubkey!("7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm"), token_a: JUP_MINT, token_b: SOL_MINT, dex: DexType::Orca },
    PoolConfig { address: pubkey!("6qgyDW4fHvpTAmfNZvPAuETEbVwRKFVAuuHfNzvEmPkY"), token_a: WIF_MINT, token_b: SOL_MINT, dex: DexType::Orca },
    PoolConfig { address: pubkey!("8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj"), token_a: JUP_MINT, token_b: USDC_MINT, dex: DexType::Raydium },

    // --- 💎 TRENDING & ARB BRIDGES ---
    PoolConfig { address: pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"), token_a: SOL_MINT, token_b: USDC_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("FxgHFpfD9kJWH2x6H5XiDjp2hQJnBGjJ3YLLPHQTwvjE"), token_a: BONK_MINT, token_b: USDC_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("319bvd2jVDbDxUr5KVcLs4wvXpkpZC3ZfCJWXh6NjH8Y"), token_a: WIF_MINT, token_b: USDC_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("6UmmUiYoBjSrhakAobJw8BvkmJtDVxaeBtbt7rxWo1mg"), token_a: RAY_MINT, token_b: USDC_MINT, dex: DexType::Raydium },
    PoolConfig { address: pubkey!("DriFtPZW76QCJj8fT4PkP8An3qcwc7pUnL9f1KxcyxBc"), token_a: SOL_MINT, token_b: DRIFT_MINT, dex: DexType::Raydium }, // DRIFT
    PoolConfig { address: pubkey!("6UYbX1x8YUcFj8YstPYiZByG7uQzAq2s46ZWphUMkjg5"), token_a: BODEN_MINT, token_b: SOL_MINT, dex: DexType::Raydium },  // BODEN
    PoolConfig { address: pubkey!("HJPjoWUrhoZzkNfRpHuieeFk9WcZWjwy6PBjZ81ngndJ"), token_a: USDC_MINT, token_b: USDT_MINT, dex: DexType::Orca },    // Stable Bridge
];

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Default)]
pub enum ExecutionMode {
    #[default]
    Simulation,      // 🛡️
    LiveMicro,       // 🧪 (Max 0.02 SOL)
    LiveProduction,  // 🚀 (Full Risk)

}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct BotConfig {
    #[serde(default)]
    pub mode: ExecutionMode,
    #[serde(alias = "RPC_URL")]
    pub rpc_url: String,
    #[serde(alias = "WS_URL")]
    pub ws_url: String,
    #[serde(alias = "JITO_URL")]
    pub jito_url: String,
    #[serde(alias = "KEYPAIR_PATH")]
    pub keypair_path: String,
    #[serde(alias = "DEFAULT_TRADE_SIZE_LAMPORTS")]
    pub default_trade_size_lamports: u64,
    #[serde(alias = "JITO_TIP_LAMPORTS")]
    pub jito_tip_lamports: u64,
    #[serde(alias = "MAX_SLIPPAGE_BPS")]
    pub max_slippage_bps: u16,
    #[serde(alias = "VOLATILITY_SENSITIVITY", default = "default_volatility_sensitivity")]
    pub volatility_sensitivity: f64,
    #[serde(alias = "MAX_SLIPPAGE_CEILING", default = "default_max_slippage_ceiling")]
    pub max_slippage_ceiling: u16,
    #[serde(alias = "JITO_TIP_PERCENTAGE", default = "default_tip_percentage")]
    pub jito_tip_percentage: f64,
    #[serde(alias = "MAX_JITO_TIP_LAMPORTS", default = "default_max_tip")]
    pub max_jito_tip_lamports: u64,
    #[serde(alias = "MONITORED_POOL_ADDRESSES")]
    pub monitored_pool_addresses: String,
    #[serde(default)]
    pub max_daily_loss_lamports: u64,
    #[serde(alias = "DISCORD_WEBHOOK")]
    pub discord_webhook: Option<String>,
    #[serde(alias = "TELEGRAM_BOT_TOKEN")]
    pub telegram_bot_token: Option<String>,
    #[serde(alias = "TELEGRAM_CHAT_ID")]
    pub telegram_chat_id: Option<String>,
    #[serde(alias = "MIN_PROFIT_THRESHOLD", default = "default_min_profit")]
    pub min_profit_threshold_lamports: u64,
    #[serde(alias = "AI_CONFIDENCE_THRESHOLD", default = "default_ai_confidence")]
    pub ai_confidence_threshold: f32,
    #[serde(alias = "KELLY_FRACTION", default = "default_kelly_fraction")]
    pub kelly_fraction: f32,
    #[serde(alias = "DATABASE_URL")]
    pub database_url: Option<String>,
    #[serde(alias = "MIN_LIQUIDITY_LAMPORTS", default = "default_min_liquidity")]
    pub min_liquidity_lamports: u64,
    #[serde(alias = "SANITY_PROFIT_FACTOR", default = "default_sanity_profit_factor")]
    pub sanity_profit_factor: u64,
    #[serde(alias = "NTFY_TOPIC")]
    pub ntfy_topic: Option<String>,
    #[serde(alias = "HELIUS_SENDER_URL")]
    pub helius_sender_url: Option<String>,
    #[serde(alias = "FEE_STRATEGY", default)]
    pub fee_strategy: FeeStrategy,
    #[serde(alias = "MAX_HOPS", default = "default_max_hops")]
    pub max_hops: u8,
    #[serde(alias = "MAX_LIQUIDITY_USD", default = "default_max_liquidity_usd")]
    pub max_liquidity_usd: u64,
    #[serde(alias = "EXCLUDED_MINTS", default = "default_excluded_mints")]
    pub excluded_mints: Vec<String>,
}

fn default_min_profit() -> u64 { 30_000 } // Lowered to 30k for better hit rate
fn default_ai_confidence() -> f32 { 0.7 } // Lowered to 0.7 (was 0.8)
fn default_kelly_fraction() -> f32 { 0.1 }
fn default_min_liquidity() -> u64 { 5_000_000_000 } // 5 SOL (was 10 SOL)
fn default_sanity_profit_factor() -> u64 { 100 } // 100x

fn default_tip_percentage() -> f64 { 0.15 }
fn default_max_tip() -> u64 { 100_000_000 } // 0.1 SOL
fn default_volatility_sensitivity() -> f64 { 1.0 }
fn default_max_slippage_ceiling() -> u16 { 200 } // 2%
fn default_max_hops() -> u8 { 5 }
fn default_max_liquidity_usd() -> u64 { 200_000 } // Cap filtering at $200k to avoid HFT
fn default_excluded_mints() -> Vec<String> {
    vec![
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT
    ]
}

impl BotConfig {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, String> {
        let s = ::config::Config::builder()
            .add_source(::config::Environment::default())
            .build()
            .map_err(|e| format!("Config Build Error: {}", e))?;

        let mut config: BotConfig = s.try_deserialize()
            .map_err(|e| format!("Config Deserialize Error: {}", e))?;

        // Handle Execution Mode from ENV directly if not in config sources
        let mode_str = env::var("EXECUTION_MODE").unwrap_or("Simulation".to_string());
        config.mode = match mode_str.as_str() {
            "Simulation" => ExecutionMode::Simulation,
            "LiveMicro" => ExecutionMode::LiveMicro,
            "LiveProduction" => ExecutionMode::LiveProduction,
            _ => return Err(format!("Invalid Execution Mode: {}", mode_str)),
        };

        // Safety Limits
        if config.mode == ExecutionMode::LiveMicro {
            config.default_trade_size_lamports = config.default_trade_size_lamports.min(20_000_000); // 0.02 SOL Hard Cap
        }

        config.max_daily_loss_lamports = 50_000_000; // 0.05 SOL

        config.validate()?;
        Ok(config)
    }

    /// Validates configuration values at startup (Fail Fast)
    pub fn validate(&self) -> Result<(), String> {
        // Validate URLs
        if !self.rpc_url.starts_with("http") {
            return Err(format!("Invalid RPC_URL: must start with http/https. Got: {}", self.rpc_url));
        }

        if self.mode == ExecutionMode::LiveProduction && self.rpc_url.contains("api.mainnet-beta.solana.com") {
            return Err("❌ DANGER: Cannot use Public RPC for Live Trading!".into());
        }
        if self.mode == ExecutionMode::LiveMicro && self.rpc_url.contains("api.mainnet-beta.solana.com") {
            tracing::warn!("⚠️  USING PUBLIC RPC FOR LIVE TRADING. Rate limits may cause missed opportunities.");
        }

        if !self.ws_url.starts_with("ws") {
            return Err(format!("Invalid WS_URL: must start with ws/wss. Got: {}", self.ws_url));
        }
        if !self.jito_url.starts_with("http") {
            return Err(format!("Invalid JITO_URL: must start with http/https. Got: {}", self.jito_url));
        }

        // Validate numeric ranges
        if self.max_slippage_bps > 10000 {
            return Err(format!("MAX_SLIPPAGE_BPS must be ≤ 10000 (100%). Got: {}", self.max_slippage_bps));
        }
        if self.max_slippage_bps == 0 {
            return Err("MAX_SLIPPAGE_BPS cannot be 0 (trades would always fail)".into());
        }

        // Validate Jito tip is reasonable
        if self.jito_tip_percentage <= 0.0 || self.jito_tip_percentage >= 1.0 {
            return Err(format!("JITO_TIP_PERCENTAGE must be between 0.0 and 1.0. Got: {}", self.jito_tip_percentage));
        }

        if self.jito_tip_lamports < 1_000 {
            tracing::warn!("⚠️  JITO_TIP_LAMPORTS (base) is very low ({}). May result in rejected bundles.", self.jito_tip_lamports);
        }

        // Validate default trade size
        if self.default_trade_size_lamports == 0 {
            return Err("DEFAULT_TRADE_SIZE_LAMPORTS cannot be 0".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_from_env() {
        env::set_var("EXECUTION_MODE", "Simulation");
        env::set_var("RPC_URL", "https://test.rpc");
        env::set_var("WS_URL", "wss://test.ws");
        env::set_var("JITO_URL", "https://test.jito");
        env::set_var("DEFAULT_TRADE_SIZE_LAMPORTS", "1000000");
        env::set_var("JITO_TIP_LAMPORTS", "1000");
        env::set_var("MAX_SLIPPAGE_BPS", "100");
        env::set_var("MONITORED_POOL_ADDRESSES", "pool1,pool2");

        let config = BotConfig::new().expect("Failed to load config");
        
        assert_eq!(config.rpc_url, "https://test.rpc");
        assert_eq!(config.ws_url, "wss://test.ws");
        assert_eq!(config.jito_url, "https://test.jito");
        assert_eq!(config.monitored_pool_addresses, "pool1,pool2");
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
