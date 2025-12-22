/// Raydium V4 AMM Swap Instruction Builder
/// 
/// This file manually constructs the raw byte instruction for a Raydium swap.
/// We avoid heavy raydium-sdk dependencies by building the instruction manually
/// (this is faster and lighter).
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::mem::size_of;

/// The Discriminator for SwapBaseIn on Raydium V4 is 9
const SWAP_BASE_IN_DISCRIMINATOR: u8 = 9;

/// Raydium V4 Program ID: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Packed struct for SwapBaseIn instruction data
/// Using packed representation ensures exact byte layout for SVM
#[repr(C, packed)]
struct SwapBaseInData {
    instruction: u8,
    amount_in: u64,
    min_amount_out: u64,
}

/// All account keys required for a Raydium V4 swap
/// Order is CRITICAL - must match Raydium program expectations exactly
#[derive(Clone, Debug)]
pub struct RaydiumSwapKeys {
    pub amm_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub serum_program_id: Pubkey,
    pub serum_market: Pubkey,
    pub serum_bids: Pubkey,
    pub serum_asks: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_coin_vault: Pubkey,
    pub serum_pc_vault: Pubkey,
    pub serum_vault_signer: Pubkey,
    pub user_source_token_account: Pubkey,
    pub user_dest_token_account: Pubkey,
    pub user_owner: Pubkey,
    pub token_program: Pubkey,
}

/// Build a Raydium V4 "Swap Base In" instruction
///
/// This constructs the raw bytes for a Raydium swap using high-performance
/// packed struct serialization.
///
/// # Arguments
/// * `keys` - All required account public keys
/// * `amount_in` - Amount of input token to swap
/// * `min_amount_out` - Minimum output (slippage protection)
///
/// # Returns
/// Complete Solana instruction ready for transaction
///
/// # Safety
/// Uses unsafe slice conversion for zero-copy serialization of POD type.
/// This is safe because SwapBaseInData is a packed, repr(C) struct with
/// only primitive types.
pub fn swap_base_in(
    keys: &RaydiumSwapKeys,
    amount_in: u64,
    min_amount_out: u64,
) -> Instruction {
    let data = SwapBaseInData {
        instruction: SWAP_BASE_IN_DISCRIMINATOR,
        amount_in,
        min_amount_out,
    };

    // Unsafe cast to byte slice (standard in high-perf Rust for POD types)
    // This avoids serialization overhead and copies bytes directly
    let data_slice = unsafe {
        std::slice::from_raw_parts(
            &data as *const _ as *const u8,
            size_of::<SwapBaseInData>(),
        )
    };

    // Account order MUST match Raydium program expectations
    // Any deviation will cause "InvalidAccountData" or transaction failure
    let accounts = vec![
        // 1. Token Program
        AccountMeta::new_readonly(keys.token_program, false),
        // 2. AMM Account (main pool state)
        AccountMeta::new(keys.amm_id, false),
        // 3. AMM Authority (PDA, not a signer)
        AccountMeta::new_readonly(keys.amm_authority, false),
        // 4. AMM Open Orders
        AccountMeta::new(keys.amm_open_orders, false),
        // 5. AMM Target Orders
        AccountMeta::new(keys.amm_target_orders, false),
        // 6. AMM Coin Vault (token A pool vault)
        AccountMeta::new(keys.amm_coin_vault, false),
        // 7. AMM PC Vault (token B pool vault)
        AccountMeta::new(keys.amm_pc_vault, false),
        // 8. Serum Program
        AccountMeta::new_readonly(keys.serum_program_id, false),
        // 9. Serum Market
        AccountMeta::new(keys.serum_market, false),
        // 10. Serum Bids
        AccountMeta::new(keys.serum_bids, false),
        // 11. Serum Asks
        AccountMeta::new(keys.serum_asks, false),
        // 12. Serum Event Queue
        AccountMeta::new(keys.serum_event_queue, false),
        // 13. Serum Coin Vault
        AccountMeta::new(keys.serum_coin_vault, false),
        // 14. Serum PC Vault
        AccountMeta::new(keys.serum_pc_vault, false),
        // 15. Serum Vault Signer (PDA)
        AccountMeta::new_readonly(keys.serum_vault_signer, false),
        // 16. User Source Token Account (will be debited)
        AccountMeta::new(keys.user_source_token_account, false),
        // 17. User Destination Token Account (will be credited)
        AccountMeta::new(keys.user_dest_token_account, false),
        // 18. User Owner (transaction signer)
        AccountMeta::new_readonly(keys.user_owner, true),
    ];

    Instruction {
        program_id: RAYDIUM_V4_PROGRAM_ID.parse().unwrap(),
        accounts,
        data: data_slice.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spl_token;
    use std::str::FromStr;

    #[test]
    fn test_instruction_layout() {
        // Verify packed struct size
        assert_eq!(size_of::<SwapBaseInData>(), 17, "SwapBaseInData should be 17 bytes: 1 + 8 + 8");

        let keys = RaydiumSwapKeys {
            amm_id: Pubkey::new_unique(),
            amm_authority: Pubkey::new_unique(),
            amm_open_orders: Pubkey::new_unique(),
            amm_target_orders: Pubkey::new_unique(),
            amm_coin_vault: Pubkey::new_unique(),
            amm_pc_vault: Pubkey::new_unique(),
            serum_program_id: Pubkey::new_unique(),
            serum_market: Pubkey::new_unique(),
            serum_bids: Pubkey::new_unique(),
            serum_asks: Pubkey::new_unique(),
            serum_event_queue: Pubkey::new_unique(),
            serum_coin_vault: Pubkey::new_unique(),
            serum_pc_vault: Pubkey::new_unique(),
            serum_vault_signer: Pubkey::new_unique(),
            user_source_token_account: Pubkey::new_unique(),
            user_dest_token_account: Pubkey::new_unique(),
            user_owner: Pubkey::default(),
            token_program: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
        };

        let ix = swap_base_in(&keys, 1_000_000, 950_000);

        // Verify instruction data
        assert_eq!(ix.data.len(), 17, "Instruction data should be 17 bytes");
        assert_eq!(ix.data[0], SWAP_BASE_IN_DISCRIMINATOR, "First byte should be discriminator");

        // Verify account count
        assert_eq!(ix.accounts.len(), 18, "Raydium swap requires exactly 18 accounts");

        // Verify signer
        assert!(ix.accounts[17].is_signer, "User owner (last account) must be signer");
        
        // Verify program ID
        assert_eq!(ix.program_id.to_string(), RAYDIUM_V4_PROGRAM_ID, "Program ID must be Raydium V4");
    }

    #[test]
    fn test_instruction_data_bytes() {
        let data = SwapBaseInData {
            instruction: 9,
            amount_in: 1000,
            min_amount_out: 950,
        };

        let bytes = unsafe {
            std::slice::from_raw_parts(
                &data as *const _ as *const u8,
                size_of::<SwapBaseInData>(),
            )
        };

        // Verify discriminator
        assert_eq!(bytes[0], 9);
        
        // Verify little-endian encoding
        assert_eq!(u64::from_le_bytes(bytes[1..9].try_into().unwrap()), 1000);
        assert_eq!(u64::from_le_bytes(bytes[9..17].try_into().unwrap()), 950);
    }
}
