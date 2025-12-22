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
    /// 
    /// # Arguments
    /// * `opportunity` - The arbitrage opportunity to evaluate
    /// 
    /// # Returns
    /// * `f32` - Confidence score between 0.0 and 1.0
    fn predict_confidence(&self, opportunity: &ArbitrageOpportunity) -> Result<f32>;
}

/// Port for bundle execution services
/// Abstracts the details of transaction submission (Jito, direct RPC, etc.)
#[async_trait::async_trait]
pub trait ExecutionPort: Send + Sync {
    /// Build transaction instructions for an arbitrage opportunity
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
/// Already exists but documented here for completeness
#[async_trait::async_trait]
pub trait BundleSimulator: Send + Sync {
    async fn simulate_bundle(
        &self,
        instructions: &[Instruction],
        payer: &Pubkey,
    ) -> std::result::Result<u64, String>;
}
