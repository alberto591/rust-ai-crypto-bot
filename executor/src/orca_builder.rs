use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::mem::size_of;
use mev_core::orca::OrcaSwapKeys;

/// Anchor Discriminator for Orca Whirlpool "swap" instruction
/// Calculated as sha256("global:swap")[..8]
const SWAP_DISCRIMINATOR: [u8; 8] = [248, 198, 158, 145, 238, 167, 205, 237];

#[repr(C, packed)]
struct SwapData {
    discriminator: [u8; 8],
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
}

pub fn swap(
    keys: &OrcaSwapKeys,
    amount: u64,
    other_amount_threshold: u64,
    mut sqrt_price_limit: u128,
    amount_specified_is_input: bool,
    a_to_b: bool,
) -> Instruction {
    // 🛡️ Safety: If limit is 0, use standard safe boundaries for direction
    if sqrt_price_limit == 0 {
        sqrt_price_limit = if a_to_b {
            mev_core::orca::MIN_SQRT_PRICE + 1
        } else {
            mev_core::orca::MAX_SQRT_PRICE - 1
        };
    }

    let data = SwapData {
        discriminator: SWAP_DISCRIMINATOR,
        amount,
        other_amount_threshold,
        sqrt_price_limit,
        amount_specified_is_input,
        a_to_b,
    };

    let data_slice = unsafe {
        std::slice::from_raw_parts(
            &data as *const _ as *const u8,
            size_of::<SwapData>(),
        )
    };

    let accounts = vec![
        AccountMeta::new_readonly(mev_core::constants::TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(keys.token_authority, true),
        AccountMeta::new(keys.whirlpool, false),
        AccountMeta::new(keys.token_owner_account_a, false),
        AccountMeta::new(keys.token_vault_a, false),
        AccountMeta::new(keys.token_owner_account_b, false),
        AccountMeta::new(keys.token_vault_b, false),
        AccountMeta::new(keys.tick_array_0, false),
        AccountMeta::new(keys.tick_array_1, false),
        AccountMeta::new(keys.tick_array_2, false),
        AccountMeta::new_readonly(keys.oracle, false),
    ];

    Instruction {
        program_id: mev_core::constants::ORCA_WHIRLPOOL_PROGRAM,
        accounts,
        data: data_slice.to_vec(),
    }
}
