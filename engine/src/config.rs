use std::env;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::pubkey;
use mev_core::DexType;
use mev_core::constants::*;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub dex: DexType,
}

pub const MONITORED_POOLS: &[PoolConfig] = &[
    // Triangle 1: SOL → JUP → USDC → SOL
    PoolConfig {
        address: pubkey!("3Fy7Py96FXtUtPYs8UPXAYqVjATbcKvN5FJGnSUNckSk"),
        token_a: SOL_MINT,
        token_b: JUP_MINT,
        dex: DexType::Raydium,
    },
    PoolConfig {
        address: pubkey!("5FsNhoCCNqv5pxzpvD8aGBCHCJPxW3FTvPVtD9n1kq4p"),
        token_a: JUP_MINT,
        token_b: USDC_MINT,
        dex: DexType::Raydium,
    },
    PoolConfig {
        address: pubkey!("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"),
        token_a: SOL_MINT,
        token_b: USDC_MINT,
        dex: DexType::Raydium,
    },
    
    // Triangle 2: SOL → RAY → USDC → SOL
    PoolConfig {
        address: pubkey!("AVs9TA4nWDzfPJE9gGVNJMVhcQy3V9PGazuz33BfG2RA"),
        token_a: RAY_MINT,
        token_b: SOL_MINT,
        dex: DexType::Raydium,
    },
    PoolConfig {
        address: pubkey!("6UmmUiYoBjSrhakAobJw8BvkmJtDVxaeBtbt7rxWo1mg"),
        token_a: RAY_MINT,
        token_b: USDC_MINT,
        dex: DexType::Raydium,
    },
    
    // Triangle 3: SOL → BONK → USDC → SOL (volatile, more opportunities)
    PoolConfig {
        address: pubkey!("Hm8DswhFx7VKXRJcdZ8bEMZvpVfQZNP6GFtHPvqgmLmD"),
        token_a: BONK_MINT,
        token_b: SOL_MINT,
        dex: DexType::Raydium,
    },
    PoolConfig {
        address: pubkey!("FxgHFpfD9kJWH2x6H5XiDjp2hQJnBGjJ3YLLPHQTwvjE"),
        token_a: BONK_MINT,
        token_b: USDC_MINT,
        dex: DexType::Raydium,
    },
    
    // Triangle 4: SOL → WIF → USDC → SOL (high volatility)
    PoolConfig {
        address: pubkey!("EP2ib6dYdEeqD8MfE2ezHCxX3kP3K2eLKkirfPm5eyMx"),
        token_a: WIF_MINT,
        token_b: SOL_MINT,
        dex: DexType::Raydium,
    },
    PoolConfig {
        address: pubkey!("319bvd2jVDbDxUr5KVcLs4wvXpkpZC3ZfCJWXh6NjH8Y"),
        token_a: WIF_MINT,
        token_b: USDC_MINT,
        dex: DexType::Raydium,
    },
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
}

fn default_tip_percentage() -> f64 { 0.5 }
fn default_max_tip() -> u64 { 100_000_000 } // 0.1 SOL
fn default_volatility_sensitivity() -> f64 { 1.0 }
fn default_max_slippage_ceiling() -> u16 { 200 } // 2%

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
