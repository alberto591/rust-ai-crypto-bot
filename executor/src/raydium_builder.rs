/// Raydium V4 AMM Swap Instruction Builder
/// 
/// This module constructs the raw Solana instructions needed to execute swaps
/// on Raydium V4 pools. The account ordering is CRITICAL - any deviation will
/// cause transaction failures.

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar,
};
use spl_token;

/// Raydium V4 AMM Program ID
pub const RAYDIUM_V4_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Pool keys required for Raydium V4 swap
/// These can be fetched from Raydium's liquidity pool data or on-chain accounts
#[derive(Debug, Clone)]
pub struct RaydiumPoolKeys {
    /// AMM ID (the pool's main account)
    pub amm_id: Pubkey,
    /// AMM authority (PDA derived from AMM ID)
    pub amm_authority: Pubkey,
    /// AMM open orders account
    pub amm_open_orders: Pubkey,
    /// AMM target orders account  
    pub amm_target_orders: Pubkey,
    /// Pool coin (token A) vault
    pub pool_coin_token_account: Pubkey,
    /// Pool PC (token B) vault
    pub pool_pc_token_account: Pubkey,
    /// Serum market program ID
    pub serum_program_id: Pubkey,
    /// Serum market account
    pub serum_market: Pubkey,
    /// Serum bids account
    pub serum_bids: Pubkey,
    /// Serum asks account
    pub serum_asks: Pubkey,
    /// Serum event queue
    pub serum_event_queue: Pubkey,
    /// Serum coin vault (base currency)
    pub serum_coin_vault_account: Pubkey,
    /// Serum PC vault (quote currency)
    pub serum_pc_vault_account: Pubkey,
    /// Serum vault signer (PDA)
    pub serum_vault_signer: Pubkey,
}

/// Instruction discriminator for SwapBaseIn
const SWAP_BASE_IN_INSTRUCTION: u8 = 9;

/// Build a Raydium V4 "Swap Base In" instruction
///
/// # Arguments
/// * `pool_keys` - Pool configuration (accounts)
/// * `user_source_token` - User's source token account (will be debited)
/// * `user_destination_token` - User's destination token account (will be credited)
/// * `user_owner` - Authority signing the transaction
/// * `amount_in` - Amount of input token to swap
/// * `min_amount_out` - Minimum amount of output token to receive (slippage protection)
///
/// # Returns
/// A complete Solana `Instruction` ready to be added to a transaction
pub fn build_raydium_swap_base_in(
    pool_keys: &RaydiumPoolKeys,
    user_source_token: Pubkey,
    user_destination_token: Pubkey,
    user_owner: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Instruction {
    // Encode instruction data
    let mut data = Vec::with_capacity(17);
    data.push(SWAP_BASE_IN_INSTRUCTION);    // Discriminator
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());

    // ⚠️ CRITICAL: Account order MUST match Raydium's program expectations
    // Any deviation will cause "InvalidAccountData" or "InvalidArgument" errors
    let accounts = vec![
        // 0. Token program
        AccountMeta::new_readonly(spl_token::ID, false),
        
        // 1. AMM ID (pool account)
        AccountMeta::new(pool_keys.amm_id, false),
        
        // 2. AMM authority (PDA, not a signer)
        AccountMeta::new_readonly(pool_keys.amm_authority, false),
        
        // 3. AMM open orders
        AccountMeta::new(pool_keys.amm_open_orders, false),
        
        // 4. AMM target orders (optional for some pools, but included for compatibility)
        AccountMeta::new(pool_keys.amm_target_orders, false),
        
        // 5. Pool coin token account (vault for token A)
        AccountMeta::new(pool_keys.pool_coin_token_account, false),
        
        // 6. Pool PC token account (vault for token B)
        AccountMeta::new(pool_keys.pool_pc_token_account, false),
        
        // 7. Serum program ID
        AccountMeta::new_readonly(pool_keys.serum_program_id, false),
        
        // 8. Serum market
        AccountMeta::new(pool_keys.serum_market, false),
        
        // 9. Serum bids
        AccountMeta::new(pool_keys.serum_bids, false),
        
        // 10. Serum asks
        AccountMeta::new(pool_keys.serum_asks, false),
        
        // 11. Serum event queue
        AccountMeta::new(pool_keys.serum_event_queue, false),
        
        // 12. Serum coin vault (base currency vault)
        AccountMeta::new(pool_keys.serum_coin_vault_account, false),
        
        // 13. Serum PC vault (quote currency vault)
        AccountMeta::new(pool_keys.serum_pc_vault_account, false),
        
        // 14. Serum vault signer (PDA derived from serum market)
        AccountMeta::new_readonly(pool_keys.serum_vault_signer, false),
        
        // 15. User source token account (will be debited)
        AccountMeta::new(user_source_token, false),
        
        // 16. User destination token account (will be credited)
        AccountMeta::new(user_destination_token, false),
        
        // 17. User transfer authority (signer)
        AccountMeta::new_readonly(user_owner, true),
    ];

    Instruction {
        program_id: RAYDIUM_V4_PROGRAM.parse().unwrap(),
        accounts,
        data,
    }
}

/// Helper function to derive AMM authority PDA
/// This is commonly needed and derived from: seeds = [AMM_ID, nonce]
pub fn get_amm_pda_authority(amm_id: &Pubkey, nonce: u8) -> Pubkey {
    Pubkey::create_program_address(
        &[&amm_id.to_bytes()[..32], &[nonce]],
        &RAYDIUM_V4_PROGRAM.parse().unwrap(),
    )
    .expect("Failed to derive AMM authority PDA")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_data_layout() {
        let pool_keys = RaydiumPoolKeys {
            amm_id: Pubkey::new_unique(),
            amm_authority: Pubkey::new_unique(),
            amm_open_orders: Pubkey::new_unique(),
            amm_target_orders: Pubkey::new_unique(),
            pool_coin_token_account: Pubkey::new_unique(),
            pool_pc_token_account: Pubkey::new_unique(),
            serum_program_id: Pubkey::new_unique(),
            serum_market: Pubkey::new_unique(),
            serum_bids: Pubkey::new_unique(),
            serum_asks: Pubkey::new_unique(),
            serum_event_queue: Pubkey::new_unique(),
            serum_coin_vault_account: Pubkey::new_unique(),
            serum_pc_vault_account: Pubkey::new_unique(),
            serum_vault_signer: Pubkey::new_unique(),
        };

        let ix = build_raydium_swap_base_in(
            &pool_keys,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000,
            900_000,
        );

        // Verify instruction data layout
        assert_eq!(ix.data[0], SWAP_BASE_IN_INSTRUCTION);
        assert_eq!(ix.data.len(), 17); // 1 + 8 + 8 bytes
        
        // Verify account count
        assert_eq!(ix.accounts.len(), 18, "Raydium V4 swap requires exactly 18 accounts");
        
        // Verify signer
        assert!(ix.accounts[17].is_signer, "User authority must be a signer");
    }
}
