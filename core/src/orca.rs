use bytemuck::{Pod, Zeroable};
use solana_sdk::pubkey::Pubkey;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Whirlpool {
    pub whirlpool_bump: [u8; 1],
    pub _padding_0: [u8; 1], // Align to 2
    pub tick_spacing: u16,
    pub tick_spacing_seed: [u8; 2], 
    pub fee_rate: u16,
    pub protocol_fee_rate: u16,
    pub _padding_2: [u8; 6], // Align 10 -> 16 bytes for [u64; 2]
    pub liquidity: [u64; 2],
    pub sqrt_price: [u64; 2],
    pub tick_current_index: i32,
    pub _padding_1: [u8; 4], // Align 52 -> 56 bytes for u64
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub token_mint_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub fee_growth_global_a: [u64; 2],
    pub token_mint_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub fee_growth_global_b: [u64; 2],
    pub reward_last_updated_timestamp: u64,
    pub reward_infos: [WhirlpoolRewardInfo; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WhirlpoolRewardInfo {
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub emissions_per_second_x64: [u64; 2],
    pub growth_global_x64: [u64; 2],
}
