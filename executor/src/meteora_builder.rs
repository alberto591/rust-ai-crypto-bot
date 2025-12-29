use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use mev_core::meteora::MeteoraSwapKeys;

pub const METEORA_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("LbSndVRSRBrs9P2ra3Sg949UasT5pU832A87W5YyWvM");

pub fn build_meteora_swap_ix(
    keys: &MeteoraSwapKeys,
    amount_in: u64,
    min_amount_out: u64,
    _x_to_y: bool,
) -> Instruction {
    let mut data = Vec::with_capacity(24);
    // Discriminator for "swap" (8 bytes)
    data.extend_from_slice(&[248, 198, 137, 192, 170, 43, 85, 170]);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(keys.dlmm_pool, false),
        AccountMeta::new_readonly(solana_sdk::pubkey!("96S9999999999999999999999999999999999999999"), false), // LbPair Authority (Placeholder)
        AccountMeta::new(keys.bin_array_bitmap_extension.unwrap_or(keys.dlmm_pool), false),
        AccountMeta::new(keys.reserve_x, false),
        AccountMeta::new(keys.reserve_y, false),
        AccountMeta::new(keys.user_token_x, false),
        AccountMeta::new(keys.user_token_y, false),
        AccountMeta::new_readonly(keys.token_x_mint, false),
        AccountMeta::new_readonly(keys.token_y_mint, false),
        AccountMeta::new_readonly(keys.oracle, false),
        AccountMeta::new_readonly(keys.user_owner, false),
        AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];

    Instruction {
        program_id: METEORA_PROGRAM_ID,
        accounts,
        data,
    }
}
