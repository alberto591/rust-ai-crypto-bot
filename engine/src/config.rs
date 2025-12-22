use serde::Deserialize;
use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize, Clone)]
pub struct BotConfig {
    pub r#rpc_url: String,
    pub ws_url: String,
    pub private_key: String,
    pub dry_run: bool,
    pub simulation: bool,
    pub pools: String,
    pub data_output_dir: String,
    pub block_engine_url: String,
}

impl BotConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start with default values or a local config file if it exists
            .add_source(File::with_name("config/default").required(false))
            // Override with environment variables
            .add_source(Environment::default().separator("__"))
            .build()?;

        s.try_deserialize()
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
        env::set_var("PRIVATE_KEY", "test_key");
        env::set_var("DRY_RUN", "true");
        env::set_var("SIMULATION", "false");
        env::set_var("POOLS", "pool1,pool2");
        env::set_var("DATA_OUTPUT_DIR", "./test_data");
        env::set_var("BLOCK_ENGINE_URL", "https://test.block.engine");

        let config = BotConfig::new().expect("Failed to load config");
        
        assert_eq!(config.rpc_url, "https://test.rpc");
        assert_eq!(config.ws_url, "wss://test.ws");
        assert_eq!(config.dry_run, true);
        assert_eq!(config.simulation, false);
    }
}
