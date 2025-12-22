use serde::Deserialize;
use config::{Config, ConfigError, Environment, File};

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct BotConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub jito_url: String,
    pub keypair_path: String,
    pub default_trade_size_lamports: u64,
    pub jito_tip_lamports: u64,
    pub max_slippage_bps: u16,
    pub monitored_pool_addresses: String,
}

impl BotConfig {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start with default values or a local config file if it exists
            .add_source(File::with_name("config/default").required(false))
            // Override with environment variables
            .add_source(Environment::default().separator("__"))
            .build()?;

        s.try_deserialize()
    }

    /// Validates configuration values at startup (Fail Fast)
    pub fn validate(&self) -> Result<(), String> {
        // Validate URLs
        if !self.rpc_url.starts_with("http") {
            return Err(format!("Invalid RPC_URL: must start with http/https. Got: {}", self.rpc_url));
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
        if self.jito_tip_lamports < 1_000 {
            tracing::warn!("⚠️  JITO_TIP_LAMPORTS is very low ({}). May result in rejected bundles.", self.jito_tip_lamports);
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
