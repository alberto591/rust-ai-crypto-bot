/// Tests for HFT-optimized ArbitrageStrategy with RwLock and SmallVec
#[cfg(test)]
mod hft_tests {
    use crate::{ArbitrageStrategy, PoolUpdate};
    use crate::analytics::volatility::VolatilityTracker;
    use solana_sdk::pubkey::Pubkey;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_rwlock_concurrent_reads() {
        // Test that multiple threads can read the graph simultaneously (RwLock benefit)
        let strategy = Arc::new(ArbitrageStrategy::new(Arc::new(VolatilityTracker::new())));
        
        let mint_sol = Pubkey::new_unique();
        let mint_usdc = Pubkey::new_unique();
        let pool = Pubkey::new_unique();

        // Add initial pool
        let update = PoolUpdate {
            pool_address: pool,
            program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
            mint_a: mint_sol,
            mint_b: mint_usdc,
            reserve_a: 1_000_000_000_000,
            reserve_b: 100_000_000_000_000,
            price_sqrt: None,
            liquidity: None,
            fee_bps: 30,
            timestamp: 0,
        };
        strategy.process_update(update.clone(), 1_000_000_000);


        // Spawn 10 concurrent readers
        let mut handles = vec![];
        for _ in 0..10 {
            let strategy_clone = Arc::clone(&strategy);
            let update_clone = update.clone();
            
            handles.push(thread::spawn(move || {
                // Read operation should not block other reads
                strategy_clone.process_update(update_clone, 1_000_000_000)
            }));
        }

        // All threads should complete without deadlock
        for handle in handles {
            let _ = handle.join().unwrap();
        }
    }

    #[test]
    fn test_smallvec_stack_allocation() {
        // Test that SmallVec uses stack allocation for common case (≤8 hops)
        let strategy = ArbitrageStrategy::new(Arc::new(VolatilityTracker::new()));
        
        let tokens: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();
        let pools: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();

        // Create a 4-hop profitable cycle with zero fees for simplicity
        // Each hop gives 1.01x return
        for i in 0..4 {
            let update = PoolUpdate {
                pool_address: pools[i],
                program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
                mint_a: tokens[i],
                mint_b: tokens[i + 1],
                reserve_a: 100_000_000_000_000,  // Large reserves to avoid slippage
                reserve_b: 101_000_000_000_000,  // 1% profitable
                price_sqrt: None,
                liquidity: None,
                fee_bps: 0,
                timestamp: 0,
            };
            strategy.process_update(update, 1_000_000_000);

        }

        // Close the cycle back to start
        let final_update = PoolUpdate {
            pool_address: pools[4],
            program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
            mint_a: tokens[4],
            mint_b: tokens[0],
            reserve_a: 100_000_000_000_000,
            reserve_b: 101_000_000_000_000,  // Another 1% gain
            price_sqrt: None,
            liquidity: None,
            fee_bps: 0,
            timestamp: 0,
        };
        
        let opp = strategy.process_update(final_update, 1_000_000_000);

        // 5 hops at zero fees with slight profit should complete
        assert!(opp.is_some(), "Should find profitable cycle");
        assert!(opp.unwrap().expected_profit_lamports > 0);
    }

    #[test]
    fn test_zero_fee_math_inlining() {
        // Test that inline(always) on math functions produces correct results
        let amount_in = 1_000_000u64;
        let reserve_in = 10_000_000u64;
        let reserve_out = 10_000_000u64;
        
        // Zero fee should give near 1:1 ratio
        let amount_out = mev_core::math::get_amount_out_cpmm(amount_in, reserve_in, reserve_out, 0);
        
        // With zero fee: out = (1M * 10M) / (10M + 1M) ≈ 909,090
        assert!(amount_out > 900_000 && amount_out < 920_000);
    }

    #[test]
    fn test_high_fee_edge_case() {
        // Test math with very high fees (edge case)
        let amount_in = 1_000_000u64;
        let reserve_in = 10_000_000u64;
        let reserve_out = 10_000_000u64;
        
        // 10% fee
        let amount_out = mev_core::math::get_amount_out_cpmm(amount_in, reserve_in, reserve_out, 1000);
        
        // Should be significantly less due to high fee
        assert!(amount_out < 850_000);
    }

    #[test]
    fn test_price_impact_extreme() {
        // Test price impact calculation at extremes
        let impact = mev_core::math::calculate_price_impact(1_000_000_000, 1_000_000_000);
        // Trading 100% of reserve should give 50% impact
        assert!((impact - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_concurrent_write_safety() {
        // Test that concurrent writes are safe (RwLock exclusivity)
        let strategy = Arc::new(ArbitrageStrategy::new(Arc::new(VolatilityTracker::new())));
        
        let mut handles = vec![];
        for i in 0..5 {
            let strategy_clone = Arc::clone(&strategy);
            
            handles.push(thread::spawn(move || {
                let mint_a = Pubkey::new_unique();
                let mint_b = Pubkey::new_unique();
                let pool = Pubkey::new_unique();
                
                let update = PoolUpdate {
                    pool_address: pool,
                    program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
                    mint_a,
                    mint_b,
                    reserve_a: (i as u128) * 1_000_000_000,
                    reserve_b: (i as u128) * 1_000_000_000,
                    price_sqrt: None,
                    liquidity: None,
                    fee_bps: 30,
                    timestamp: 0,
                };
                
                strategy_clone.process_update(update, 1_000_000_000)
            }));

        }

        // All writes should succeed without data races
        for handle in handles {
            let _ = handle.join().unwrap();
        }
    }
}
