#[cfg(test)]
mod whitelist_tests {
    use crate::safety::token_validator::TokenSafetyChecker;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_whitelist_bypass_usdc() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com");
        let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let pool = Pubkey::new_unique();
        
        let is_safe = checker.is_safe_to_trade(&usdc, &pool).await.unwrap();

        assert!(is_safe, "USDC should bypass all safety checks");
    }

    #[tokio::test]
    async fn test_whitelist_bypass_usdt() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com");
        let usdt = Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap();
        let pool = Pubkey::new_unique();
        
        let is_safe = checker.is_safe_to_trade(&usdt, &pool).await.unwrap();

        assert!(is_safe, "USDT should bypass all safety checks");
    }

    #[tokio::test]
    async fn test_whitelist_bypass_wrapped_sol() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com");
        let wsol = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let pool = Pubkey::new_unique();
        
        let is_safe = checker.is_safe_to_trade(&wsol, &pool).await.unwrap();

        assert!(is_safe, "Wrapped SOL should bypass all safety checks");
    }

    #[tokio::test]
    async fn test_non_whitelisted_token_runs_checks() {
        let checker = TokenSafetyChecker::new("https://api.mainnet-beta.solana.com");
        let random_token = Pubkey::new_unique();
        let pool = Pubkey::new_unique();

        // This will fail due to RPC but we're just testing that it doesn't bypass
        // In a real scenario, non-whitelisted tokens should go through full validation
        let result = checker.is_safe_to_trade(&random_token, &pool).await;

        // Since RPC fails for non-existent token, it should return an error
        // The key is that it ATTEMPTED validation instead of bypassing
        assert!(result.is_err(), "Non-whitelisted tokens should run full safety checks");
    }
}
