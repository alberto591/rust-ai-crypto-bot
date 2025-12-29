use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    transaction::VersionedTransaction,
    message::v0::Message,
    pubkey::Pubkey,
};
use tracing::{debug, error};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("RPC error: {0}")]
    RpcError(#[from] solana_client::client_error::ClientError),
    #[error("Simulation failed: {0}")]
    Failed(String),
    #[error("Message compile error: {0}")]
    CompileError(#[from] solana_sdk::message::CompileError),
    #[error("Transaction creation error: {0}")]
    TransactionError(#[from] solana_sdk::transaction::TransactionError),
}

pub struct Simulator {
    rpc_client: Arc<RpcClient>,
    cached_blockhash: std::sync::Mutex<Option<(solana_sdk::hash::Hash, std::time::Instant)>>,
}

#[async_trait::async_trait]
impl strategy::BundleSimulator for Simulator {
    async fn simulate_bundle(
        &self, 
        instructions: &[Instruction],
        payer: &Pubkey,
    ) -> Result<u64, String> {
        self.simulate_bundle_internal(instructions, payer)
            .await
            .map_err(|e| e.to_string())
    }
}

impl Simulator {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { 
            rpc_client,
            cached_blockhash: std::sync::Mutex::new(None),
        }
    }

    pub async fn simulate_bundle_internal(
        &self, 
        instructions: &[Instruction],
        payer: &Pubkey,
    ) -> Result<u64, SimulationError> {
        debug!("Simulating bundle with {} instructions", instructions.len());

        // 🛡️ BATCH OPTIMIZATION: Cache blockhash for 30s to save RPC credits
        let recent_blockhash = {
            let mut cache = self.cached_blockhash.lock().unwrap();
            if let Some((hash, ts)) = *cache {
                if ts.elapsed() < std::time::Duration::from_secs(30) {
                    hash
                } else {
                    let hash = self.rpc_client.get_latest_blockhash()
                        .map_err(SimulationError::RpcError)?;
                    *cache = Some((hash, std::time::Instant::now()));
                    hash
                }
            } else {
                let hash = self.rpc_client.get_latest_blockhash()
                    .map_err(SimulationError::RpcError)?;
                *cache = Some((hash, std::time::Instant::now()));
                hash
            }
        };
        
        let message = Message::try_compile(
            payer,
            instructions,
            &[],
            recent_blockhash,
        )?;
        
        let tx = VersionedTransaction::try_new::<[&dyn solana_sdk::signer::Signer; 0]>(
            solana_sdk::message::VersionedMessage::V0(message),
            &[], 
        ).map_err(|e| SimulationError::Failed(e.to_string()))?;

        // 2. Call simulate_transaction
        let result = self.rpc_client.simulate_transaction(&tx)
            .map_err(SimulationError::RpcError)?;

        if let Some(err) = result.value.err {
            error!("Simulation REVERTED: {:?}", err);
            return Err(SimulationError::Failed(format!("{:?}", err)));
        }

        let units_consumed = result.value.units_consumed.unwrap_or(0);
        debug!("Simulation SUCCEEDED: {} units consumed", units_consumed);

        Ok(units_consumed)
    }
}
