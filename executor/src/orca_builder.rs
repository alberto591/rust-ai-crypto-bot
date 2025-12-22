/// Orca Whirlpool Swap Instruction Builder
///
/// Orca Whirlpools use concentrated liquidity (CLMM) which is more complex than
/// traditional AMMs. This module provides a simplified swap builder.

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use spl_token;

/// Orca Whirlpool Program ID
pub const ORCA_WHIRLPOOL_PROGRAM: &str = "whirLbMiqkh6thXv7uBToywS9Bn1McGQ669YUsbAHQi";

/// Whirlpool pool configuration
#[derive(Debug, Clone)]
pub struct WhirlpoolKeys {
    /// Whirlpool account (main pool state)
    pub whirlpool: Pubkey,
    /// Token A mint
    pub token_mint_a: Pubkey,
    /// Token B mint
    pub token_mint_b: Pubkey,
    /// Token A vault (pool's token A account)
    pub token_vault_a: Pubkey,
    /// Token B vault (pool's token B account)
    pub token_vault_b: Pubkey,
    /// Tick array 0 (price range data)
    pub tick_array_0: Pubkey,
    /// Tick array 1
    pub tick_array_1: Pubkey,
    /// Tick array 2
    pub tick_array_2: Pubkey,
    /// Oracle account (for price feeds, may be optional)
    pub oracle: Pubkey,
}

/// Swap instruction discriminator for Orca Whirlpool
/// Note: This is a placeholder - actual discriminator should be verified from Orca docs
const WHIRLPOOL_SWAP_INSTRUCTION: &[u8] = &[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]; // Anchor discriminator for "swap"

/// Build an Orca Whirlpool swap instruction
///
/// # Arguments
/// * `pool_keys` - Whirlpool configuration
/// * `user_token_a` - User's token A account
/// * `user_token_b` - User's token B account
/// * `user_authority` - Owner/signer
/// * `amount` - Amount to swap
/// * `other_amount_threshold` - Minimum output (slippage protection)
/// * `sqrt_price_limit` - Price limit (u128, pass u128::MAX for no limit)
/// * `amount_specified_is_input` - True if `amount` is input, false if output
/// * `a_to_b` - True for A→B swap, false for B→A
///
/// # Returns
/// Whirlpool swap instruction
pub fn build_whirlpool_swap(
    pool_keys: &WhirlpoolKeys,
    user_token_a: Pubkey,
    user_token_b: Pubkey,
    user_authority: Pubkey,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Instruction {
    // Encode instruction data
    // Format: discriminator (8 bytes) + amount (8) + other_amount_threshold (8) + sqrt_price_limit (16) + flags (2 bools)
    let mut data = Vec::with_capacity(42);
    data.extend_from_slice(WHIRLPOOL_SWAP_INSTRUCTION); // 8 bytes
    data.extend_from_slice(&amount.to_le_bytes());      // 8 bytes
    data.extend_from_slice(&other_amount_threshold.to_le_bytes()); // 8 bytes
    data.extend_from_slice(&sqrt_price_limit.to_le_bytes()); // 16 bytes
    data.push(amount_specified_is_input as u8);         // 1 byte
    data.push(a_to_b as u8);                             // 1 byte

    // Account ordering for Whirlpool swap
    let accounts = vec![
        // 0. Token program
        AccountMeta::new_readonly(spl_token::ID, false),
        
        // 1. Token authority (signer)
        AccountMeta::new_readonly(user_authority, true),
        
        // 2. Whirlpool account
        AccountMeta::new(pool_keys.whirlpool, false),
        
        // 3. User token account A
        AccountMeta::new(user_token_a, false),
        
        // 4. User token account B
         AccountMeta::new(user_token_b, false),
        
        // 5. Token vault A (pool's vault)
        AccountMeta::new(pool_keys.token_vault_a, false),
        
        // 6. Token vault B (pool's vault)
        AccountMeta::new(pool_keys.token_vault_b, false),
        
        // 7-9. Tick arrays (concentrated liquidity price ranges)
        AccountMeta::new(pool_keys.tick_array_0, false),
        AccountMeta::new(pool_keys.tick_array_1, false),
        AccountMeta::new(pool_keys.tick_array_2, false),
        
        // 10. Oracle (price feed, can be whirlpool account if no external oracle)
        AccountMeta::new_readonly(pool_keys.oracle, false),
    ];

    Instruction {
        program_id: ORCA_WHIRLPOOL_PROGRAM.parse().unwrap(),
        accounts,
        data,
    }
}

/// Simplified swap builder with sensible defaults
/// Use this for most cases where you just want to swap A→B with slippage protection
pub fn build_simple_whirlpool_swap(
    pool_keys: &WhirlpoolKeys,
    user_token_a: Pubkey,
    user_token_b: Pubkey,
    user_authority: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Instruction {
    build_whirlpool_swap(
        pool_keys,
        user_token_a,
        user_token_b,
        user_authority,
        amount_in,
        min_amount_out,
        u128::MAX, // No price limit
        true,      // Amount specified is input
        a_to_b,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whirlpool_instruction() {
        let pool_keys = WhirlpoolKeys {
            whirlpool: Pubkey::new_unique(),
            token_mint_a: Pubkey::new_unique(),
            token_mint_b: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            tick_array_0: Pubkey::new_unique(),
            tick_array_1: Pubkey::new_unique(),
            tick_array_2: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
        };

        let ix = build_simple_whirlpool_swap(
            &pool_keys,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            1_000_000,
            950_000,
            true,
        );

        // Verify account count
        assert_eq!(ix.accounts.len(), 11, "Whirlpool swap requires 11 accounts");
        
        // Verify signer
        assert!(ix.accounts[1].is_signer, "Token authority must be a signer");
        
        // Verify data layout
        assert!(ix.data.len() >= 8, "Must contain at least the discriminator");
    }
}
