#[cfg(test)]
mod whitelist_tests {
    use crate::safety::token_validator::TokenSafetyChecker;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_whitelist_bypass_usdc() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com", 10_000_000_000);
        let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let pool = Pubkey::new_unique();
        
        let is_safe = checker.is_safe_to_trade(&usdc, &pool).await.unwrap();

        assert!(is_safe, "USDC should bypass all safety checks");
    }

    #[tokio::test]
    async fn test_whitelist_bypass_usdt() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com", 10_000_000_000);
        let usdt = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();
        let pool = Pubkey::new_unique();
        
        let is_safe = checker.is_safe_to_trade(&usdt, &pool).await.unwrap();

        assert!(is_safe, "USDT should bypass all safety checks");
    }

    #[tokio::test]
    async fn test_whitelist_bypass_wrapped_sol() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com", 10_000_000_000);
        let wsol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let pool = Pubkey::new_unique();
        
        let is_safe = checker.is_safe_to_trade(&wsol, &pool).await.unwrap();

        assert!(is_safe, "Wrapped SOL should bypass all safety checks");
    }

    #[tokio::test]
    async fn test_non_whitelisted_token_runs_checks() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com", 10_000_000_000);
        let random_token = Pubkey::new_unique();
        let pool = Pubkey::new_unique();

        // This will fail due to RPC but we're just testing that it doesn't bypass
        // In a real scenario, non-whitelisted tokens should go through full validation
        let result = checker.is_safe_to_trade(&random_token, &pool).await;

        // Since RPC fails for non-existent token, it should return an error
        // The key is that it ATTEMPTED validation instead of bypassing
        assert!(result.is_err(), "Non-whitelisted tokens should run full safety checks");
    }

    #[tokio::test]
    async fn test_safety_check_caching() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com", 10_000_000_000);
        let token = Pubkey::new_unique();
        let pool = Pubkey::new_unique();

        // 1. Manually seed the cache -> Should pass without RPC
        checker.safe_cache.insert(token, std::time::Instant::now());
        
        // This would normally fail (garbage RPC) or panic if it tried to call RPC
        // But since it's cached, it should return true immediately
        let is_safe = checker.is_safe_to_trade(&token, &pool).await.unwrap();
        assert!(is_safe, "Cached token should pass safety check immediately");

        // 2. Test Blacklist Caching
        let bad_token = Pubkey::new_unique();
        checker.blacklist.insert(bad_token, std::time::Instant::now());

        let is_safe_bad = checker.is_safe_to_trade(&bad_token, &pool).await.unwrap();
        assert!(!is_safe_bad, "Blacklisted token should fail properly");
    }
}
