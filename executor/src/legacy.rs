/// Legacy RPC Transaction Executor
///
/// This module sends transactions to the public mempool (RPC) instead of the 
/// Jito Block Engine. This is your "Testing Mode" executor for development
/// and non-MEV-sensitive operations.
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use std::error::Error;

/// Legacy executor using standard Solana RPC
pub struct LegacyExecutor {
    client: RpcClient,
    payer: solana_sdk::signature::Keypair,
    payer_pubkey: solana_sdk::pubkey::Pubkey,
    key_provider: Option<std::sync::Arc<dyn strategy::ports::PoolKeyProvider>>,
}

impl LegacyExecutor {
    /// Create a new legacy executor
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint (e.g., "https://api.mainnet-beta.solana.com")
    ///
    /// # Returns
    /// Configured executor with confirmed commitment level
    pub fn new(
        rpc_url: &str,
        payer: solana_sdk::signature::Keypair,
        key_provider: Option<std::sync::Arc<dyn strategy::ports::PoolKeyProvider>>,
    ) -> Self {
        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );
        let payer_pubkey = payer.pubkey();
        Self { client, payer, payer_pubkey, key_provider }
    }

    /// Execute a standard transaction via RPC
    ///
    /// # Arguments
    /// * `payer` - Transaction fee payer and signer
    /// * `ixs` - Instructions to execute atomically
    ///
    /// # Returns
    /// Transaction signature string on success
    ///
    /// # Errors
    /// Returns error if:
    /// - Failed to get recent blockhash
    /// - Transaction building failed
    /// - Transaction confirmation failed
    ///
    /// # Note
    /// Uses `send_and_confirm_transaction` for testing reliability.
    /// In production, consider using `send_transaction` with a custom
    /// confirmation loop for better performance.
    pub fn execute_standard_tx(
        &self,
        payer: &Keypair,
        ixs: &[Instruction],
    ) -> Result<String, Box<dyn Error>> {
        // 1. Get latest blockhash (recent check required for all transactions)
        let recent_blockhash = self.client.get_latest_blockhash()?;

        // 2. Build Transaction
        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&payer.pubkey()),
            &[payer], // Signers
            recent_blockhash,
        );

        // 🛡️ SAFETY ADDITION: PRE-FLIGHT SIMULATION
        // Ask the node: "If I ran this, would it work?"
        tracing::debug!("🕵️ Simulating transaction...");
        let simulation = self.client.simulate_transaction(&tx)?;
        
        if let Some(err) = simulation.value.err {
            // If simulation fails, WE ABORT. We do not send it.
            tracing::error!("❌ Simulation Failed: {:?}", err);
            tracing::error!("   Logs: {:?}", simulation.value.logs);
            return Err("Pre-flight simulation failed. Trade aborted safely.".into());
        }
        
        tracing::info!("✅ Simulation Passed! Gas used: {}", simulation.value.units_consumed.unwrap_or(0));

        // 3. Send and Confirm
        // We use send_and_confirm for testing reliability. 
        // In production, use send_transaction with a custom confirmation loop.
        let signature = self.client.send_and_confirm_transaction(&tx)?;

        Ok(signature.to_string())
    }

    /// Execute transaction without waiting for confirmation (fire-and-forget)
    ///
    /// Faster but riskier - transaction may still fail after this returns success.
    pub fn execute_no_confirm(
        &self,
        payer: &Keypair,
        ixs: &[Instruction],
    ) -> Result<String, Box<dyn Error>> {
        let recent_blockhash = self.client.get_latest_blockhash()?;

        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&payer.pubkey()),
            &[payer],
            recent_blockhash,
        );

        let signature = self.client.send_transaction(&tx)?;

        Ok(signature.to_string())
    }

    /// Execute with custom commitment level
    pub fn execute_with_commitment(
        &self,
        payer: &Keypair,
        ixs: &[Instruction],
        commitment: CommitmentConfig,
    ) -> Result<String, Box<dyn Error>> {
        let recent_blockhash = self.client
            .get_latest_blockhash_with_commitment(commitment)?
            .0;

        let tx = Transaction::new_signed_with_payer(
            ixs,
            Some(&payer.pubkey()),
            &[payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction_with_spinner_and_commitment(
            &tx,
            commitment,
        )?;

        Ok(signature.to_string())
    }

    /// Get reference to underlying RPC client for advanced usage
    pub fn client(&self) -> &RpcClient {
        &self.client
    }
}

#[async_trait::async_trait]
impl strategy::ports::PoolKeyProvider for LegacyExecutor {
    async fn get_swap_keys(&self, pool_address: &solana_sdk::pubkey::Pubkey) -> anyhow::Result<mev_core::raydium::RaydiumSwapKeys> {
        if let Some(provider) = &self.key_provider {
            provider.get_swap_keys(pool_address).await
        } else {
            Err(anyhow::anyhow!("No PoolKeyProvider configured for LegacyExecutor"))
        }
    }

    async fn get_orca_keys(&self, pool_address: &solana_sdk::pubkey::Pubkey) -> anyhow::Result<mev_core::orca::OrcaSwapKeys> {
        if let Some(provider) = &self.key_provider {
            provider.get_orca_keys(pool_address).await
        } else {
            Err(anyhow::anyhow!("No PoolKeyProvider configured for LegacyExecutor"))
        }
    }
}

#[async_trait::async_trait]
impl strategy::ports::ExecutionPort for LegacyExecutor {
    async fn build_bundle_instructions(
        &self,
        opportunity: mev_core::ArbitrageOpportunity,
        _tip_lamports: u64,
        max_slippage_bps: u16,
    ) -> anyhow::Result<Vec<Instruction>> {
        let mut ixs = Vec::new();
        let mut current_amount_in = opportunity.input_amount;
        let min_amount_out = (opportunity.input_amount as u128 * (10000 - max_slippage_bps) as u128 / 10000) as u64;

        let num_steps = opportunity.steps.len();

        for (i, step) in opportunity.steps.iter().enumerate() {
            let is_last_step = i == num_steps - 1;
            let step_min_out = if is_last_step { min_amount_out } else { 0 };

            if step.program_id == mev_core::constants::RAYDIUM_V4_PROGRAM {
                let keys = strategy::ports::PoolKeyProvider::get_swap_keys(self, &step.pool).await?;
                ixs.push(crate::raydium_builder::swap_base_in(
                    &keys,
                    current_amount_in,
                    step_min_out, 
                ));
            } else if step.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM {
                let keys = strategy::ports::PoolKeyProvider::get_orca_keys(self, &step.pool).await?;
                let a_to_b = step.input_mint == keys.mint_a;
                ixs.push(crate::orca_builder::swap(
                    &keys,
                    current_amount_in,
                    step_min_out,
                    0,
                    true,
                    a_to_b,
                ));
            }
            
            // Track amount for multi-hop
            current_amount_in = step.expected_output;
        }

        Ok(ixs)
    }

    async fn build_and_send_bundle(
        &self,
        opportunity: mev_core::ArbitrageOpportunity,
        _recent_blockhash: solana_sdk::hash::Hash,
        tip_lamports: u64,
        max_slippage_bps: u16,
    ) -> anyhow::Result<String> {
        let ixs = self.build_bundle_instructions(opportunity, tip_lamports, max_slippage_bps).await?;
        
        match self.execute_standard_tx(&self.payer, &ixs) {
            Ok(sig) => Ok(sig),
            Err(e) => Err(anyhow::anyhow!("Legacy execution failed: {}", e)),
        }
    }

    fn pubkey(&self) -> &solana_sdk::pubkey::Pubkey {
        &self.payer_pubkey
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{system_instruction, pubkey::Pubkey};

    #[test]
    fn test_executor_creation() {
        let executor = LegacyExecutor::new("https://api.mainnet-beta.solana.com", Keypair::new(), None);
        // Should create without errors
        assert!(executor.client().commitment() == CommitmentConfig::confirmed());
    }

    #[test]
    #[ignore] // Requires live RPC connection
    fn test_execute_transfer() {
        // This test requires a live RPC connection and funded account
        // Run with: cargo test --package executor -- --ignored

        let payer = Keypair::new();
        let executor = LegacyExecutor::new("https://api.mainnet-beta.solana.com", Keypair::from_bytes(&payer.to_bytes()).unwrap(), None);
        
        let instruction = system_instruction::transfer(
            &payer.pubkey(),
            &Pubkey::new_unique(),
            1_000_000, // 0.001 SOL
        );

        // Expected to fail due to insufficient funds, but tests the builder logic
        let result = executor.execute_standard_tx(&payer, &[instruction]);
        
        assert!(result.is_err(), "Should fail without funding");
    }
}
