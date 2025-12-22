// Port Definitions for Hexagonal Architecture
// These traits define the boundaries between application and infrastructure layers

use anyhow::Result;
use mev_core::ArbitrageOpportunity;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, hash::Hash};

/// Port for AI/ML prediction services
/// Allows swapping between different model implementations (ONNX, remote API, mock, etc.)
#[async_trait::async_trait]
pub trait AIModelPort: Send + Sync {
    /// Predict confidence score for an arbitrage opportunity
    fn predict_confidence(&self, opportunity: &ArbitrageOpportunity) -> Result<f32>;
}

/// Port for resolving pool keys required for instruction building
/// Decouples the executor from specific RPC or local database clients
#[async_trait::async_trait]
pub trait PoolKeyProvider: Send + Sync {
    async fn get_swap_keys(&self, pool_address: &Pubkey) -> Result<mev_core::raydium::RaydiumSwapKeys>;
}

/// Port for bundle execution services
/// Abstracts the details of transaction submission (Jito, direct RPC, etc.)
#[async_trait::async_trait]
pub trait ExecutionPort: Send + Sync {
    /// Build instructions for an opportunity (for simulation or external use)
    async fn build_bundle_instructions(
        &self,
        opportunity: ArbitrageOpportunity,
        tip_lamports: u64,
    ) -> Result<Vec<Instruction>>;

    /// Build and send a complete bundle to the network
    async fn build_and_send_bundle(
        &self,
        opportunity: ArbitrageOpportunity,
        recent_blockhash: Hash,
        tip_lamports: u64,
    ) -> Result<String>;

    /// Get the public key of the execution account
    fn pubkey(&self) -> &Pubkey;
}

/// Port for bundle simulation services
#[async_trait::async_trait]
pub trait BundleSimulator: Send + Sync {
    async fn simulate_bundle(
        &self,
        instructions: &[Instruction],
        payer: &Pubkey,
    ) -> std::result::Result<u64, String>;
}
