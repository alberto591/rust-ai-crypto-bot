use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    program_error::ProgramError,
};
use mev_core::raydium::RaydiumSwapKeys;
use std::mem::size_of;

// Raydium Swap Instruction Discriminator (9) for 'swap_base_in'
const SWAP_BASE_IN_OPCODE: u8 = 9;

pub fn build_raydium_swap(
    pool_keys: &RaydiumSwapKeys,
    user_source: Pubkey,
    user_destination: Pubkey,
    user_owner_pubkey: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<Instruction, ProgramError> {
    
    // 1. Define Accounts (18 accounts for Raydium V4)
    // Ref: https://github.com/raydium-io/raydium-contract-instructions/blob/master/amm_instruction.rs
    // https://github.com/raydium-io/raydium-ui-v3/blob/master/src/utils/pools/utils.ts
    
    let accounts = vec![
        // 0. Token Program
        AccountMeta::new_readonly(pool_keys.token_program, false),
        // 1. AMM ID
        AccountMeta::new(pool_keys.amm_id, false),
        // 2. AMM Authority
        AccountMeta::new_readonly(pool_keys.amm_authority, false),
        // 3. AMM Open Orders
        AccountMeta::new(pool_keys.amm_open_orders, false),
        // 4. AMM Target Orders
        AccountMeta::new(pool_keys.amm_target_orders, false),
        // 5. AMM One Coin Vault (Base/Coin)
        AccountMeta::new(pool_keys.amm_coin_vault, false),
        // 6. AMM Two Coin Vault (Quote/Pc)
        AccountMeta::new(pool_keys.amm_pc_vault, false),
        // 7. Serum Program ID
        AccountMeta::new_readonly(pool_keys.serum_program_id, false),
        // 8. Serum Market
        AccountMeta::new(pool_keys.serum_market, false),
        // 9. Serum Bids
        AccountMeta::new(pool_keys.serum_bids, false),
        // 10. Serum Asks
        AccountMeta::new(pool_keys.serum_asks, false),
        // 11. Serum Event Queue
        AccountMeta::new(pool_keys.serum_event_queue, false),
        // 12. Serum Coin Vault
        AccountMeta::new(pool_keys.serum_coin_vault, false),
        // 13. Serum Pc Vault
        AccountMeta::new(pool_keys.serum_pc_vault, false),
        // 14. Serum Vault Signer
        AccountMeta::new_readonly(pool_keys.serum_vault_signer, false),
        // 15. User Source Token Account
        AccountMeta::new(user_source, false),
        // 16. User Dest Token Account
        AccountMeta::new(user_destination, false),
        // 17. User Owner
        AccountMeta::new_readonly(user_owner_pubkey, true),
    ];
    
    // 2. Data
    // u8: 9
    // u64: amount_in
    // u64: min_amount_out
    let mut data = Vec::with_capacity(1 + 8 + 8);
    data.push(SWAP_BASE_IN_OPCODE);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());

    Ok(Instruction {
        program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
        accounts,
        data,
    })
}

/// HFT Optimization: Patches an existing instruction template to avoid allocations.
pub fn patch_raydium_swap(
    ix: &mut Instruction,
    amount_in: u64,
    min_amount_out: u64,
    user_source: Pubkey,
    user_destination: Pubkey,
) {
    // 1. Patch Data (Opcode 9 at 0, amount_in at 1, min_amount_out at 9)
    if ix.data.len() >= 17 {
        ix.data[1..9].copy_from_slice(&amount_in.to_le_bytes());
        ix.data[9..17].copy_from_slice(&min_amount_out.to_le_bytes());
    }
    
    // 2. Patch User Accounts (15 and 16)
    if ix.accounts.len() >= 17 {
        ix.accounts[15].pubkey = user_source;
        ix.accounts[16].pubkey = user_destination;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_patch_raydium_swap() {
        let keys = RaydiumSwapKeys {
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
            user_source_token_account: Pubkey::new_unique(),
            user_destination_token_account: Pubkey::new_unique(),
            user_source_owner: Pubkey::new_unique(),
        };

        let mut ix = build_raydium_swap(&keys, 1000, 900).unwrap();
        
        // Initial state
        assert_eq!(u64::from_le_bytes(ix.data[9..17].try_into().unwrap()), 1000);
        assert_eq!(u64::from_le_bytes(ix.data[17..25].try_into().unwrap()), 900);

        // Patch
        patch_raydium_swap(&mut ix, 5000, 4500);

        // Verified state
        assert_eq!(u64::from_le_bytes(ix.data[9..17].try_into().unwrap()), 5000);
        assert_eq!(u64::from_le_bytes(ix.data[17..25].try_into().unwrap()), 4500);
    }
}
