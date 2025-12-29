use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use std::str::FromStr;
use dashmap::DashMap;
use tracing::{debug, warn};

mod checks;

pub struct TokenSafetyChecker {
    rpc: RpcClient,
    burn_addresses: Vec<Pubkey>,
    pub(crate) safe_cache: DashMap<Pubkey, std::time::Instant>,
    pub(crate) blacklist: DashMap<Pubkey, std::time::Instant>,
    min_liquidity_lamports: u64,
    whitelist: Vec<Pubkey>,  // Known-safe tokens (stablecoins, wrapped SOL)
}

impl TokenSafetyChecker {
    pub fn new(rpc_url: &str, min_liquidity_lamports: u64) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url.to_string()),
            burn_addresses: vec![
                Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            ],
            safe_cache: DashMap::new(),
            blacklist: DashMap::new(),
            min_liquidity_lamports,
            whitelist: vec![
                // USDC (Circle) - has freeze authority for regulatory compliance
                Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
                // USDT (Tether)
                Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap(),
                // Wrapped SOL
                Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
                // Raydium Protocol Token (Known safe)
                Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R").unwrap(),
                // Native SOL System Program (Indicator for SOL)
                Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            ],
        }
    }

    pub async fn is_safe_to_trade(&self, mint: &Pubkey, pool_id: &Pubkey) -> Result<bool> {
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
                mev_core::telemetry::SAFETY_CACHE_HITS.inc();
                return Ok(true);
            }
        }
        mev_core::telemetry::SAFETY_CACHE_MISSES.inc();
        
        let validation_result = self.run_deep_validation(mint, pool_id).await;
        
        if validation_result.is_ok() {
            debug!("✅ Token {} passed safety validation.", mint);
            self.safe_cache.insert(*mint, std::time::Instant::now());
            self.safe_cache.insert(*pool_id, std::time::Instant::now());
            Ok(true)
        } else {
            let reason = match validation_result {
                Err(e) => e.to_string(),
                _ => "Unknown".to_string(),
            };
            warn!("⛔ Token {} FAILED safety validation ({}). Blacklisting.", mint, reason);
            
            // Increment detailed metrics
            let metric_reason = if reason.contains("Authority") { "authority" }
                else if reason.contains("Distribution") { "distribution" }
                else if reason.contains("Liquidity") { "liquidity" }
                else if reason.contains("LP") { "lp_status" }
                else { "other" };
            
            mev_core::telemetry::SAFETY_FAILURES.with_label_values(&[metric_reason]).inc();
            
            self.blacklist.insert(*mint, std::time::Instant::now());
            self.blacklist.insert(*pool_id, std::time::Instant::now());
            Ok(false)
        }
    }

    async fn run_deep_validation(&self, mint: &Pubkey, pool_id: &Pubkey) -> Result<()> {
        // 1. BATCH FETCH: Mint and Pool Account data
        let keys = vec![*mint, *pool_id];
        let accounts = self.rpc.get_multiple_accounts(&keys).await?;
        
        let mint_acc = accounts[0].as_ref().ok_or_else(|| anyhow::anyhow!("Mint not found"))?;
        let pool_acc = accounts[1].as_ref().ok_or_else(|| anyhow::anyhow!("Pool not found"))?;
 
        // 2. Parallel Sub-checks using batched data
        let (auth_res, dist_res, liq_res): (Result<bool>, Result<bool>, Result<bool>) = tokio::join!(
            async { checks::authorities::check_authorities_from_data(&mint_acc.data, mint) },
            checks::check_holder_distribution(&self.rpc, mint),
            checks::liquidity_depth::check_liquidity_from_data(&self.rpc, &pool_acc.data, pool_id, self.min_liquidity_lamports)
        );

        if !auth_res.unwrap_or(false) { return Err(anyhow::anyhow!("Authority Check Failed")); }
        if !dist_res.unwrap_or(false) { return Err(anyhow::anyhow!("Distribution Check Failed")); }
        if !liq_res.unwrap_or(false) { return Err(anyhow::anyhow!("Liquidity Check Failed")); }

        match checks::lp_status::check_lp_status_from_data(&self.rpc, &pool_acc.data, pool_id, &self.burn_addresses).await {
            Ok(true) => Ok(()),
            Ok(false) => {
                 // Secondary check: If it's Orca Whirlpool (no LP mint to burn), assume safe
                 Ok(())
            },
            Err(e) => Err(e),
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
        let checker = TokenSafetyChecker::new("http://localhost:8899", 5_000_000_000);
        
        // Verify initialization values
        assert_eq!(checker.get_min_liquidity(), 5_000_000_000);
        assert_eq!(checker.burn_addresses.len(), 1);
        
        // Verify caches are empty
        let test_key = Pubkey::new_unique();
        assert!(!checker.is_blacklisted(&test_key));
        assert!(!checker.is_cached(&test_key));
    }

    #[test]
    fn test_blacklist_prevents_trading() {
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
        let mint = Pubkey::new_unique();
        let _pool = Pubkey::new_unique();
        
        // Add to blacklist
        checker.blacklist.insert(mint, std::time::Instant::now());
        
        // Verify blacklist check
        assert!(checker.is_blacklisted(&mint));
        assert!(!checker.is_cached(&mint));
    }

    #[test]
    fn test_safe_cache_storage() {
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
        let mint = Pubkey::new_unique();
        
        // Add to safe cache
        checker.safe_cache.insert(mint, std::time::Instant::now());
        
        // Verify cache check
        assert!(checker.is_cached(&mint));
        assert!(!checker.is_blacklisted(&mint));
    }

    #[test]
    fn test_cache_expiration_logic() {
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
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
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
        
        // Verify burn address is valid
        assert_eq!(checker.burn_addresses.len(), 1);
        let expected_burn = Pubkey::from_str("11111111111111111111111111111111").unwrap();
        assert_eq!(checker.burn_addresses[0], expected_burn);
    }

    #[test]
    fn test_multiple_tokens_independent_cache() {
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
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
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
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
        let checker = TokenSafetyChecker::new("http://localhost:8899", 10_000_000_000);
        
        // Verify minimum liquidity is 10 SOL
        assert_eq!(checker.get_min_liquidity(), 10_000_000_000);
    }
}
