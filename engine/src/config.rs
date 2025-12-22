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
