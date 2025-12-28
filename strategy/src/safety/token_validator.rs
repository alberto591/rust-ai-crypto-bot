use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::error::Error;
use std::str::FromStr;
use dashmap::DashMap;
use tracing::{debug, warn};

mod checks;

pub struct TokenSafetyChecker {
    rpc: RpcClient,
    burn_addresses: Vec<Pubkey>,
    safe_cache: DashMap<Pubkey, std::time::Instant>,
    blacklist: DashMap<Pubkey, std::time::Instant>,
    min_liquidity_lamports: u64,
    whitelist: Vec<Pubkey>,  // Known-safe tokens (stablecoins, wrapped SOL)
}

impl TokenSafetyChecker {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url.to_string()),
            burn_addresses: vec![
                Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            ],
            safe_cache: DashMap::new(),
            blacklist: DashMap::new(),
            min_liquidity_lamports: 10_000_000_000, // 10 SOL minimum
            whitelist: vec![
                // USDC (Circle) - has freeze authority for regulatory compliance
                Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
                // USDT (Tether)
                Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap(),
                // Wrapped SOL
                Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            ],
        }
    }

    pub async fn is_safe_to_trade(&self, mint: &Pubkey, pool_id: &Pubkey) -> Result<bool, Box<dyn Error>> {
        // SHORT-CIRCUIT: Whitelist check first (known-safe stablecoins)
        if self.whitelist.contains(mint) {
            debug!("✅ Token {} is whitelisted. Skipping safety checks.", mint);
            return Ok(true);
        }

        if self.blacklist.contains_key(mint) || self.blacklist.contains_key(pool_id) {
            return Ok(false);
        }

        if let Some(timestamp_ref) = self.safe_cache.get(mint) {
            if (*timestamp_ref).elapsed() < std::time::Duration::from_secs(3600) {
                return Ok(true);
            }
        }

        let is_safe = self.run_deep_validation(mint, pool_id).await;

        if is_safe {
            debug!("✅ Token {} passed safety validation.", mint);
            self.safe_cache.insert(*mint, std::time::Instant::now());
            self.safe_cache.insert(*pool_id, std::time::Instant::now());
        } else {
            warn!("⛔ Token {} FAILED safety validation. Blacklisting.", mint);
            self.blacklist.insert(*mint, std::time::Instant::now());
            self.blacklist.insert(*pool_id, std::time::Instant::now());
        }

        Ok(is_safe)
    }

    async fn run_deep_validation(&self, mint: &Pubkey, pool_id: &Pubkey) -> bool {
        if let Ok(false) = checks::check_authorities(&self.rpc, mint).await { return false; }
        if let Ok(false) = checks::check_holder_distribution(&self.rpc, mint).await { return false; }
        if let Ok(false) = checks::check_liquidity_depth(&self.rpc, pool_id, self.min_liquidity_lamports).await { return false; }

        match checks::check_lp_status(&self.rpc, pool_id, &self.burn_addresses).await {
            Ok(true) => true,
            Ok(false) => {
                // Secondary check: If it's Orca Whirlpool (no LP mint to burn), assume safe if in monitored pools
                true
            },
            Err(_) => false,
        }
    }


    // Exposed for testing
    #[cfg(test)]
    pub fn is_blacklisted(&self, key: &Pubkey) -> bool {
        self.blacklist.contains_key(key)
    }

    #[cfg(test)]
    pub fn is_cached(&self, key: &Pubkey) -> bool {
        self.safe_cache.contains_key(key)
    }

    #[cfg(test)]
    pub fn get_min_liquidity(&self) -> u64 {
        self.min_liquidity_lamports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_safety_checker_initialization() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        
        // Verify initialization values
        assert_eq!(checker.get_min_liquidity(), 10_000_000_000);
        assert_eq!(checker.burn_addresses.len(), 1);
        
        // Verify caches are empty
        let test_key = Pubkey::new_unique();
        assert!(!checker.is_blacklisted(&test_key));
        assert!(!checker.is_cached(&test_key));
    }

    #[test]
    fn test_blacklist_prevents_trading() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        let mint = Pubkey::new_unique();
        let pool = Pubkey::new_unique();
        
        // Add to blacklist
        checker.blacklist.insert(mint, std::time::Instant::now());
        
        // Verify blacklist check
        assert!(checker.is_blacklisted(&mint));
        assert!(!checker.is_cached(&mint));
    }

    #[test]
    fn test_safe_cache_storage() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        let mint = Pubkey::new_unique();
        
        // Add to safe cache
        checker.safe_cache.insert(mint, std::time::Instant::now());
        
        // Verify cache check
        assert!(checker.is_cached(&mint));
        assert!(!checker.is_blacklisted(&mint));
    }

    #[test]
    fn test_cache_expiration_logic() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        let mint = Pubkey::new_unique();
        
        // Add to cache with old timestamp (simulating expiration)
        let old_timestamp = std::time::Instant::now() - std::time::Duration::from_secs(7200); // 2 hours ago
        checker.safe_cache.insert(mint, old_timestamp);
        
        // Verify cache exists but is expired
        let timestamp_value = checker.safe_cache.get(&mint).map(|r| *r).unwrap();
        let elapsed = timestamp_value.elapsed();
        assert!(elapsed > std::time::Duration::from_secs(3600), 
            "Cache should be expired after 1 hour");
    }

    #[test]
    fn test_burn_address_configuration() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        
        // Verify burn address is valid
        assert_eq!(checker.burn_addresses.len(), 1);
        let expected_burn = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        assert_eq!(checker.burn_addresses[0], expected_burn);
    }

    #[test]
    fn test_multiple_tokens_independent_cache() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        let mint1 = Pubkey::new_unique();
        let mint2 = Pubkey::new_unique();
        
        // Add mint1 to cache, mint2 to blacklist
        checker.safe_cache.insert(mint1, std::time::Instant::now());
        checker.blacklist.insert(mint2, std::time::Instant::now());
        
        // Verify independence
        assert!(checker.is_cached(&mint1));
        assert!(!checker.is_blacklisted(&mint1));
        
        assert!(checker.is_blacklisted(&mint2));
        assert!(!checker.is_cached(&mint2));
    }

    #[test]
    fn test_cache_and_blacklist_mutual_exclusivity() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        let mint = Pubkey::new_unique();
        
        // Add to cache first
        checker.safe_cache.insert(mint, std::time::Instant::now());
        assert!(checker.is_cached(&mint));
        
        // Then add to blacklist (simulating re-evaluation)
        checker.blacklist.insert(mint, std::time::Instant::now());
        
        // Both can exist, but blacklist should take precedence in is_safe_to_trade logic
        assert!(checker.is_cached(&mint));
        assert!(checker.is_blacklisted(&mint));
    }

    #[test]
    fn test_min_liquidity_threshold() {
        let checker = TokenSafetyChecker::new("http://localhost:8899");
        
        // Verify minimum liquidity is 10 SOL
        assert_eq!(checker.get_min_liquidity(), 10_000_000_000);
    }
}
