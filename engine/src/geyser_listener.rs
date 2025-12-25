use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;
use tokio::sync::mpsc;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use tracing::{info, warn, error};
use mev_core::MarketUpdate;

/// Yellowstone gRPC listener for high-speed account updates
pub struct GeyserListener {
    endpoint: String,
    token: Option<String>,
}

impl GeyserListener {
    pub fn new(endpoint: String, token: Option<String>) -> Self {
        Self { endpoint, token }
    }

    /// Start listening to account updates via gRPC
    pub async fn start(
        &self,
        pool_addresses: Vec<Pubkey>,
        tx: mpsc::Sender<MarketUpdate>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting Yellowstone gRPC listener for {} pools", pool_addresses.len());

        // Connect to gRPC endpoint
        let mut client = GeyserGrpcClient::connect(
            self.endpoint.clone(),
            self.token.clone(),
            None,
        )?;

        // Build subscription request
        let mut accounts_filter = HashMap::new();
        
        // Subscribe to all monitored pool addresses
        for (i, pool_address) in pool_addresses.iter().enumerate() {
            accounts_filter.insert(
                format!("pool_{}", i),
                SubscribeRequestFilterAccounts {
                    account: vec![pool_address.to_string()],
                    owner: vec![],
                    filters: vec![],
                },
            );
        }

        let request = SubscribeRequest {
            slots: HashMap::new(),
            accounts: accounts_filter,
            transactions: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(CommitmentLevel::Confirmed as i32),
            accounts_data_slice: vec![],
            ping: None,
        };

        info!("📡 Subscribing to gRPC account updates...");
        let (mut subscribe_tx, mut stream) = client.subscribe().await?;
        
        // Send subscription request
        subscribe_tx.send(request).await?;
        info!("✅ gRPC subscription established");

        // Process incoming updates
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    if let Some(update) = msg.update_oneof {
                        match update {
                            subscribe_update::UpdateOneof::Account(account_update) => {
                                self.process_account_update(account_update, &tx).await;
                            }
                            subscribe_update::UpdateOneof::Ping(_) => {
                                // Keep-alive ping, no action needed
                            }
                            _ => {
                                // Ignore other update types (transactions, slots, etc.)
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("❌ gRPC stream error: {}", e);
                    return Err(Box::new(e));
                }
            }
        }

        warn!("⚠️ gRPC stream ended");
        Ok(())
    }

    async fn process_account_update(
        &self,
        account_update: SubscribeUpdateAccount,
        tx: &mpsc::Sender<MarketUpdate>,
    ) {
        if let Some(account_info) = account_update.account {
            let pubkey_str = bs58::encode(&account_info.pubkey).into_string();
            
            // Parse account data (this will reuse existing Raydium/Orca parsing logic)
            if let Ok(pubkey) = pubkey_str.parse::<Pubkey>() {
                // TODO: Parse pool data based on owner (Raydium vs Orca)
                // For now, log the update
                info!("📊 gRPC update for pool: {}", pubkey);
                
                // This would integrate with existing pool parsing logic
                // Example: parse AmmInfo, Whirlpool, etc.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geyser_listener_creation() {
        let listener = GeyserListener::new(
            "http://localhost:10000".to_string(),
            Some("test-token".to_string()),
        );
        assert_eq!(listener.endpoint, "http://localhost:10000");
    }
}
