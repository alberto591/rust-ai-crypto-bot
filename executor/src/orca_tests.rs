#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;
    use mev_core::orca::{Whirlpool, OrcaSwapKeys};
    use std::str::FromStr;

    #[test]
    fn test_orca_whirlpool_decoding() {
        // Create a dummy Whirlpool account data (653 bytes)
        let mut data = vec![0u8; 653];
        
        // Mock token mints
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        
        // Set mints at correct offsets
        data[101..133].copy_from_slice(&mint_a.to_bytes());
        data[181..213].copy_from_slice(&mint_b.to_bytes());
        
        // Set sqrt_price (X64)
        let price = 1.2345f64;
        let sqrt_p = (price.sqrt() * (1u128 << 64) as f64) as u128;
        data[65..81].copy_from_slice(&sqrt_p.to_le_bytes());
        
        // Set liquidity
        let liq = 1_000_000_000u128;
        data[49..65].copy_from_slice(&liq.to_le_bytes());
        
        // Set tick spacing and current tick
        data[41..43].copy_from_slice(&64u16.to_le_bytes());
        data[81..85].copy_from_slice(&(-1000i32).to_le_bytes());

        let whirlpool: &Whirlpool = bytemuck::try_from_bytes(&data).unwrap();
        
        assert_eq!(whirlpool.token_mint_a(), mint_a);
        assert_eq!(whirlpool.token_mint_b(), mint_b);
        assert_eq!(whirlpool.sqrt_price(), sqrt_p);
        assert_eq!(whirlpool.liquidity(), liq);
        assert_eq!(whirlpool.tick_spacing(), 64);
        assert_eq!(whirlpool.tick_current_index(), -1000);
    }

    #[test]
    fn test_tick_array_derivation_multi_spacing() {
        let pool_id = Pubkey::new_unique();
        let program_id = mev_core::constants::ORCA_WHIRLPOOL_PROGRAM;

        // Spacing 64 (Standard)
        // ticks_in_array = 88 * 64 = 5632
        assert_eq!(OrcaSwapKeys::get_tick_array_start_index(-1000, 64), -5632);
        assert_eq!(OrcaSwapKeys::get_tick_array_start_index(500, 64), 0);
        assert_eq!(OrcaSwapKeys::get_tick_array_start_index(6000, 64), 5632);

        // Spacing 1 (Lowest)
        // ticks_in_array = 88 * 1 = 88
        assert_eq!(OrcaSwapKeys::get_tick_array_start_index(-100, 1), -176); // floor(-100/88) = -2, -2*88 = -176
        assert_eq!(OrcaSwapKeys::get_tick_array_start_index(100, 1), 88);   // floor(100/88) = 1, 1*88 = 88

        // Spacing 128 (High)
        // ticks_in_array = 88 * 128 = 11264
        assert_eq!(OrcaSwapKeys::get_tick_array_start_index(20000, 128), 11264); // floor(20000/11264) = 1
        
        let pda = OrcaSwapKeys::derive_tick_array_pda(&pool_id, -5632, &program_id);
        assert!(pda != Pubkey::default());
    }

    #[test]
    fn test_orca_swap_instruction_layout() {
        let keys = OrcaSwapKeys {
            whirlpool: Pubkey::new_unique(),
            mint_a: Pubkey::new_unique(),
            mint_b: Pubkey::new_unique(),
            token_authority: Pubkey::new_unique(),
            token_owner_account_a: Pubkey::new_unique(),
            token_vault_a: Pubkey::new_unique(),
            token_owner_account_b: Pubkey::new_unique(),
            token_vault_b: Pubkey::new_unique(),
            tick_array_0: Pubkey::new_unique(),
            tick_array_1: Pubkey::new_unique(),
            tick_array_2: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
        };

        let amount = 1_000_000_000;
        let threshold = 990_000_000;
        let sqrt_price_limit = 0; // Should be auto-fixed by builder
        
        let ix = crate::orca_builder::swap(
            &keys,
            amount,
            threshold,
            sqrt_price_limit,
            true,
            true, // a_to_b
        );

        // Verify account count (11 accounts)
        assert_eq!(ix.accounts.len(), 11);
        assert!(ix.accounts[1].is_signer); // token_authority
        assert_eq!(ix.accounts[2].pubkey, keys.whirlpool);
        
        // Verify Data Layout
        // discriminator: 8, amount: 8, threshold: 8, sqrt_limit: 16, spec_input: 1, a_to_b: 1
        // Total = 8+8+8+16+1+1 = 42 bytes
        assert_eq!(ix.data.len(), 42); 
        
        // Check discriminator
        assert_eq!(&ix.data[0..8], &[248, 198, 158, 145, 238, 167, 205, 237]);
        
        // Check amounts
        assert_eq!(u64::from_le_bytes(ix.data[8..16].try_into().unwrap()), amount);
        assert_eq!(u64::from_le_bytes(ix.data[16..24].try_into().unwrap()), threshold);
        
        // Check auto-filled sqrt_price_limit for a_to_b (should be MIN_SQRT_PRICE + 1)
        let limit = u128::from_le_bytes(ix.data[24..40].try_into().unwrap());
        assert_eq!(limit, mev_core::orca::MIN_SQRT_PRICE + 1);
        
        // Check booleans
        assert_eq!(ix.data[40], 1); // spec_input
        assert_eq!(ix.data[41], 1); // a_to_b
    }
}
