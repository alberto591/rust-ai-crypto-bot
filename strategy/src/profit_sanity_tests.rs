#[cfg(test)]
mod profit_sanity_tests {
    // Test the profit sanity check logic (from strategy/src/lib.rs lines 88-102)
    // Logic: reject if profit > initial_amount / 10 (10%)
    
    fn calc_max_reasonable_profit(input: u64) -> u64 {
        input / 10  // 10% threshold
    }
    
    fn is_profit_reasonable(profit: u64, input: u64) -> bool {
        profit <= calc_max_reasonable_profit(input)
    }

    #[test]
    fn test_profit_sanity_check_reasonable_profit() {
        let input = 1_000_000_000; // 1 SOL
        let profit = 50_000_000;    // 0.05 SOL (5%)
        
        assert!(is_profit_reasonable(profit, input), "5% profit should be considered reasonable");
    }

    #[test]
    fn test_profit_sanity_check_at_threshold() {
        let input = 1_000_000_000; // 1 SOL
        let profit = 100_000_000;   // 0.1 SOL (10% - exactly at threshold)
        
        assert!(is_profit_reasonable(profit, input), "10% profit should be at threshold");
    }

    #[test]
    fn test_profit_sanity_check_unrealistic_profit() {
        let input = 1_000_000_000;  // 1 SOL
        let profit = 540_865_614;   // 540 SOL (54000% - clearly unrealistic)
        
        assert!(!is_profit_reasonable(profit, input), "540 SOL profit on 1 SOL input should be rejected");
    }

    #[test]
    fn test_profit_sanity_check_edge_case_small_input() {
        let input = 100_000;        // 0.0001 SOL (very small)
        let profit = 20_000;        // 0.00002 SOL (20%)
        
        assert!(!is_profit_reasonable(profit, input), "20% should be rejected even for small amounts");
    }

    #[test]
    fn test_profit_sanity_check_percentage_calculation() {
        let input = 1_000_000_000; // 1 SOL
        
        // Test various percentage scenarios
        let test_cases = vec![
            (10_000_000, 1, true),    // 1% - should pass
            (50_000_000, 5, true),    // 5% - should pass
            (100_000_000, 10, true),  // 10% - should pass (at threshold)
            (150_000_000, 15, false), // 15% - should fail
            (200_000_000, 20, false), // 20% - should fail
            (500_000_000, 50, false), // 50% - should fail
        ];

        for (profit, expected_pct, should_pass) in test_cases {
            let actual_pct = (profit * 100) / input;
            
            assert_eq!(actual_pct, expected_pct, "Percentage calculation should be accurate");
            assert_eq!(is_profit_reasonable(profit, input), should_pass, 
                "{}% profit should {}", expected_pct, if should_pass { "pass" } else { "fail" });
        }
    }

    #[test]
    fn test_profit_sanity_check_zero_profit() {
        let input = 1_000_000_000; // 1 SOL
        let profit = 0;
        
        assert!(is_profit_reasonable(profit, input), "Zero profit should technically pass sanity check");
    }

    #[test]
    fn test_max_reasonable_calculation() {
        assert_eq!(calc_max_reasonable_profit(1_000_000), 100_000);
        assert_eq!(calc_max_reasonable_profit(1_000_000_000), 100_000_000);
        assert_eq!(calc_max_reasonable_profit(10_000), 1_000);
    }
}
