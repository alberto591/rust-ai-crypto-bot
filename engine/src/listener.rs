use std::sync::Arc;
use crossbeam::channel::Sender;
use tracing::{info, error, debug};
use mev_core::{PoolUpdate, raydium::AmmInfo};
use solana_sdk::pubkey::Pubkey;
use solana_client::{pubsub_client::PubsubClient, rpc_config::RpcAccountInfoConfig};
use solana_sdk::commitment_config::CommitmentConfig;

pub struct SolanaListener {
    sender: Sender<PoolUpdate>,
}

impl SolanaListener {
    pub fn new(sender: Sender<PoolUpdate>) -> Self {
        Self { sender }
    }

    pub async fn start(&self, grpc_url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Solana gRPC Listener on {}", grpc_url);
        // ... (existing gRPC logic)
        Ok(())
    }

    pub fn inject_simulation_update(&self, update: PoolUpdate) {
        debug!("Injecting simulation update for pool: {}", update.pool_address);
        if let Err(e) = self.sender.send(update) {
            error!("Failed to send simulated update: {}", e);
        }
    }
}

pub fn start_listener(rpc_url: String, pool_address: Pubkey, sender: Sender<PoolUpdate>) {
    info!("Starting Raydium PubSub Listener for pool: {}", pool_address);

    let pubsub_rpc_url = rpc_url.replace("https://", "wss://").replace("http://", "ws://");
    
    std::thread::spawn(move || {
        let (mut _subscription, receiver) = match PubsubClient::account_subscribe(
            &pubsub_rpc_url,
            &pool_address,
            Some(RpcAccountInfoConfig {
                commitment: Some(CommitmentConfig::processed()),
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            }),
        ) {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to subscribe to account: {}", e);
                return;
            }
        };

        loop {
            match receiver.recv() {
                Ok(response) => {
                    let data: Vec<u8> = match response.value.data.decode() {
                        Some(d) => d,
                        None => continue,
                    };

                    // Use pod_read_unaligned to handle potential misalignment in Vec<u8> buffer
                    if data.len() == std::mem::size_of::<AmmInfo>() {
                         let amm_info: AmmInfo = bytemuck::pod_read_unaligned(&data);
                         let update = PoolUpdate {
                            pool_address,
                            program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
                            mint_a: amm_info.base_mint,
                            mint_b: amm_info.quote_mint,
                            reserve_a: amm_info.base_reserve as u128,
                            reserve_b: amm_info.quote_reserve as u128,
                            fee_bps: 30, // Standard Raydium fee
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        };

                        if let Err(e) = sender.send(update) {
                            error!("Failed to send PoolUpdate: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("PubSub receiver error: {}", e);
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod listener_tests {
    use super::*;
}
