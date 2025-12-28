use bytemuck::{Pod, Zeroable};
use solana_sdk::pubkey::Pubkey;

pub const MIN_SQRT_PRICE: u128 = 4295048016;
pub const MAX_SQRT_PRICE: u128 = 79226673515401241271192636570;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Whirlpool {
    pub data: [u8; 653],
}

unsafe impl Zeroable for Whirlpool {}
unsafe impl Pod for Whirlpool {}

impl Whirlpool {
    #[inline(always)]
    pub fn token_mint_a(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[101..133].try_into().unwrap())
    }

    #[inline(always)]
    pub fn token_mint_b(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[181..213].try_into().unwrap())
    }

    #[inline(always)]
    pub fn token_vault_a(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[133..165].try_into().unwrap())
    }

    #[inline(always)]
    pub fn token_vault_b(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[213..245].try_into().unwrap())
    }

    #[inline(always)]
    pub fn sqrt_price(&self) -> u128 {
        u128::from_le_bytes(self.data[65..81].try_into().unwrap())
    }

    #[inline(always)]
    pub fn liquidity(&self) -> u128 {
        u128::from_le_bytes(self.data[49..65].try_into().unwrap())
    }

    #[inline(always)]
    pub fn tick_current_index(&self) -> i32 {
        i32::from_le_bytes(self.data[81..85].try_into().unwrap())
    }

    #[inline(always)]
    pub fn tick_spacing(&self) -> u16 {
        u16::from_le_bytes(self.data[41..43].try_into().unwrap())
    }

    #[inline(always)]
    pub fn fee_rate(&self) -> u16 {
        u16::from_le_bytes(self.data[45..47].try_into().unwrap())
    }

    /// Calculate the current price in the pool (quote/base)
    /// For concentrated liquidity, price = (sqrt_price / 2^64)^2
    pub fn calculate_price(&self) -> f64 {
        let sqrt_price_f64 = self.sqrt_price() as f64 / (1u128 << 64) as f64;
        sqrt_price_f64 * sqrt_price_f64
    }

    /// Estimate output amount for a given input (with slippage)
    /// This is a simplified calculation - production should use exact tick math
    pub fn estimate_swap_output(
        &self,
        amount_in: u64,
        a_to_b: bool,
    ) -> Result<u64, &'static str> {
        let liquidity = self.liquidity();
        if liquidity == 0 {
            return Err("Pool has no liquidity");
        }

        let sqrt_price = self.sqrt_price();
        let fee_rate = self.fee_rate();
        
        // Apply fee
        let amount_in_after_fee = amount_in as u128 * (1_000_000 - fee_rate as u128) / 1_000_000;
        
        // Simplified constant product approximation
        // Real implementation should walk through ticks
        let sqrt_price_f64 = sqrt_price as f64 / (1u128 << 64) as f64;
        let price = sqrt_price_f64 * sqrt_price_f64;
        
        let amount_out = if a_to_b {
            (amount_in_after_fee as f64 * price) as u64
        } else {
            (amount_in_after_fee as f64 / price) as u64
        };
        
        Ok(amount_out)
    }

    pub fn to_pool_update(&self, pool_address: Pubkey, program_id: Pubkey, timestamp: u64) -> crate::PoolUpdate {
        crate::PoolUpdate {
            pool_address,
            program_id,
            mint_a: self.token_mint_a(),
            mint_b: self.token_mint_b(),
            reserve_a: 0, // Not used for CLMM
            reserve_b: 0, // Not used for CLMM
            price_sqrt: Some(self.sqrt_price()),
            liquidity: Some(self.liquidity()),
            fee_bps: self.fee_rate(), // Orca fee_rate is in bps
            timestamp,
        }
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrcaSwapKeys {
    pub whirlpool: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub token_authority: Pubkey,
    pub token_owner_account_a: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_owner_account_b: Pubkey,
    pub token_vault_b: Pubkey,
    pub tick_array_0: Pubkey,
    pub tick_array_1: Pubkey,
    pub tick_array_2: Pubkey,
    pub oracle: Pubkey,
}

impl OrcaSwapKeys {
    pub const TICKS_PER_ARRAY: i32 = 88;

    pub fn get_tick_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = Self::TICKS_PER_ARRAY * tick_spacing as i32;
        ((tick_index as f64 / ticks_in_array as f64).floor() as i32) * ticks_in_array
    }

    pub fn derive_tick_array_pda(
        whirlpool: &Pubkey,
        start_tick_index: i32,
        program_id: &Pubkey,
    ) -> Pubkey {
        let (pda, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                whirlpool.as_ref(),
                start_tick_index.to_string().as_bytes(),
            ],
            program_id,
        );
        pda
    }
}

use serde::{Serialize, Deserialize};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whirlpool_layout_exhaustive() {
        let mut data = [0u8; 653];
        
        // Mocking values based on official Whirlpool layout
        // https://github.com/orca-so/whirlpools/blob/main/programs/whirlpool/src/state/whirlpool.rs
        
        let liquidity: u128 = 100_000_000_000;
        data[49..65].copy_from_slice(&liquidity.to_le_bytes());
        
        let sqrt_price: u128 = 18446744073709551616; // 1.0 in X64
        data[65..81].copy_from_slice(&sqrt_price.to_le_bytes());
        
        let tick_current_index: i32 = -450;
        data[81..85].copy_from_slice(&tick_current_index.to_le_bytes());
        
        let tick_spacing: u16 = 64;
        data[41..43].copy_from_slice(&tick_spacing.to_le_bytes());
        
        let fee_rate: u16 = 3000; // 30 bps
        data[45..47].copy_from_slice(&fee_rate.to_le_bytes());
        
        let mint_a = Pubkey::new_unique();
        data[101..133].copy_from_slice(&mint_a.to_bytes());
        
        let mint_b = Pubkey::new_unique();
        data[181..213].copy_from_slice(&mint_b.to_bytes());
        
        let vault_a = Pubkey::new_unique();
        data[133..165].copy_from_slice(&vault_a.to_bytes());
        
        let vault_b = Pubkey::new_unique();
        data[213..245].copy_from_slice(&vault_b.to_bytes());

        let whirlpool: &Whirlpool = bytemuck::from_bytes(&data);
        
        assert_eq!(whirlpool.liquidity(), liquidity);
        assert_eq!(whirlpool.sqrt_price(), sqrt_price);
        assert_eq!(whirlpool.tick_current_index(), tick_current_index);
        assert_eq!(whirlpool.tick_spacing(), tick_spacing);
        assert_eq!(whirlpool.fee_rate(), fee_rate);
        assert_eq!(whirlpool.token_mint_a(), mint_a);
        assert_eq!(whirlpool.token_mint_b(), mint_b);
        assert_eq!(whirlpool.token_vault_a(), vault_a);
        assert_eq!(whirlpool.token_vault_b(), vault_b);
        
        // Verify math helpers
        assert_eq!(whirlpool.calculate_price(), 1.0);
    }

    #[test]
    fn test_tick_array_math() {
        // Test case: current_tick = -450, spacing = 64
        // TICKS_PER_ARRAY = 88
        // ticks_in_array = 88 * 64 = 5632
        // start_index = floor(-450 / 5632) * 5632 = -1 * 5632 = -5632
        let start = OrcaSwapKeys::get_tick_array_start_index(-450, 64);
        assert_eq!(start, -5632);
        
        // Test case: current_tick = 6000, spacing = 64
        // floor(6000 / 5632) * 5632 = 1 * 5632 = 5632
        let start_pos = OrcaSwapKeys::get_tick_array_start_index(6000, 64);
        assert_eq!(start_pos, 5632);

        // Test case: current_tick = 500, spacing = 64
        // floor(500 / 5632) * 5632 = 0 * 5632 = 0
        let start_zero = OrcaSwapKeys::get_tick_array_start_index(500, 64);
        assert_eq!(start_zero, 0);
    }

    #[test]
    fn test_pda_derivation_smoke() {
        let pool = Pubkey::new_unique();
        let program = crate::constants::ORCA_WHIRLPOOL_PROGRAM;
        let pda = OrcaSwapKeys::derive_tick_array_pda(&pool, -5632, &program);
        assert!(pda != Pubkey::default());
    }
}
