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
}

impl LegacyExecutor {
    /// Create a new legacy executor
    ///
    /// # Arguments
    /// * `rpc_url` - Solana RPC endpoint (e.g., "https://api.mainnet-beta.solana.com")
    ///
    /// # Returns
    /// Configured executor with confirmed commitment level
    pub fn new(rpc_url: &str) -> Self {
        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );
        Self { client }
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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{system_instruction, pubkey::Pubkey};

    #[test]
    fn test_executor_creation() {
        let executor = LegacyExecutor::new("https://api.mainnet-beta.solana.com");
        // Should create without errors
        assert!(executor.client().commitment() == CommitmentConfig::confirmed());
    }

    #[test]
    #[ignore] // Requires live RPC connection
    fn test_execute_transfer() {
        // This test requires a live RPC connection and funded account
        // Run with: cargo test --package executor -- --ignored

        let executor = LegacyExecutor::new("https://api.mainnet-beta.solana.com");
        let payer = Keypair::new();
        
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
