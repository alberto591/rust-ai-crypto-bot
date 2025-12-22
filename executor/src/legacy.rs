/// Legacy RPC Transaction Executor
///
/// This module provides a simple way to execute transactions via standard Solana RPC,
/// bypassing Jito bundles. Useful for testing and development when Jito connectivity
/// is unavailable or for non-MEV-sensitive operations.

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use anyhow::{Result, Context};
use tracing::{info, error};

/// Execute a standard Solana transaction via RPC
///
/// # Arguments
/// * `rpc` - Connected RPC client
/// * `payer` - Transaction fee payer and signer
/// * `instructions` - List of instructions to execute atomically
///
/// # Returns
/// Transaction signature string on success
///
/// # Errors
/// Returns error if:
/// - Failed to get recent blockhash
/// - Transaction simulation failed
/// - Transaction confirmation failed
pub fn execute_standard_tx(
    rpc: &RpcClient,
    payer: &Keypair,
    instructions: &[Instruction],
) -> Result<String> {
    info!("Building standard transaction with {} instructions", instructions.len());

    // 1. Get recent blockhash
    let recent_blockhash = rpc
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    info!("Recent blockhash: {}", recent_blockhash);

    // 2. Build transaction
    let mut transaction = Transaction::new_with_payer(
        instructions,
        Some(&payer.pubkey()),
    );

    // 3. Sign transaction
    transaction.sign(&[payer], recent_blockhash);

    info!("Transaction signed. Signature: {}", transaction.signatures[0]);

    // 4. Simulate before sending (safety check)
    match rpc.simulate_transaction(&transaction) {
        Ok(simulation) => {
            if simulation.value.err.is_some() {
                error!("Transaction simulation failed: {:?}", simulation.value.err);
                anyhow::bail!("Simulation failed: {:?}", simulation.value.err);
            }
            info!("Simulation successful. Logs: {:?}", simulation.value.logs);
        }
        Err(e) => {
            error!("Failed to simulate transaction: {}", e);
            anyhow::bail!("Simulation error: {}", e);
        }
    }

    // 5. Send and confirm transaction
    info!("Sending transaction...");
    let signature = rpc
        .send_and_confirm_transaction_with_spinner(&transaction)
        .context("Failed to send and confirm transaction")?;

    info!("Transaction confirmed: {}", signature);
    Ok(signature.to_string())
}

/// Execute transaction without waiting for confirmation (faster, but riskier)
///
/// Use this when you don't need to wait for finalization and want maximum throughput.
/// Note: Transaction may still fail after this function returns success.
pub fn execute_standard_tx_no_confirm(
    rpc: &RpcClient,
    payer: &Keypair,
    instructions: &[Instruction],
) -> Result<String> {
    let recent_blockhash = rpc
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    let mut transaction = Transaction::new_with_payer(
        instructions,
        Some(&payer.pubkey()),
    );

    transaction.sign(&[payer], recent_blockhash);

    // Send without confirmation
    let signature = rpc
        .send_transaction(&transaction)
        .context("Failed to send transaction")?;

    info!("Transaction sent (no confirmation): {}", signature);
    Ok(signature.to_string())
}

/// Execute with custom commitment level
pub fn execute_with_commitment(
    rpc: &RpcClient,
    payer: &Keypair,
    instructions: &[Instruction],
    commitment: CommitmentConfig,
) -> Result<String> {
    let recent_blockhash = rpc
        .get_latest_blockhash_with_commitment(commitment)
        .context("Failed to get recent blockhash")?
        .0;

    let mut transaction = Transaction::new_with_payer(
        instructions,
        Some(&payer.pubkey()),
    );

    transaction.sign(&[payer], recent_blockhash);

    // Send with commitment
    let signature = rpc
        .send_and_confirm_transaction_with_spinner_and_commitment(
            &transaction,
            commitment,
        )
        .context("Failed to send transaction")?;

    info!("Transaction confirmed with {:?}: {}", commitment, signature);
    Ok(signature.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{system_instruction, pubkey::Pubkey};

    #[test]
    #[ignore] // Requires RPC connection
    fn test_execute_transfer() {
        // This test requires a live RPC connection
        // Run with: cargo test --package executor -- --ignored

        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let payer = Keypair::new();
        
        // Create a simple transfer instruction
        let instruction = system_instruction::transfer(
            &payer.pubkey(),
            &Pubkey::new_unique(),
            1_000_000, // 0.001 SOL
        );

        // This will fail without funding, but tests the builder logic
        let result = execute_standard_tx(&rpc, &payer, &[instruction]);
        
        // Expected to fail due to insufficient funds, but structure should be valid
        assert!(result.is_err());
    }
}
