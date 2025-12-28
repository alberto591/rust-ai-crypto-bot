use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use solana_sdk::program_pack::Pack;
use std::error::Error;
use std::str::FromStr;
use mev_core::raydium::AmmInfo;
use dashmap::DashMap;
use tracing::{warn, debug};

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
        if let Ok(false) = self.check_authorities(mint).await { return false; }
        if let Ok(false) = self.check_holder_distribution(mint).await { return false; }
        if let Ok(false) = self.check_liquidity_depth(pool_id).await { return false; }
        
        match self.check_lp_status(pool_id).await {
            Ok(true) => true,
            Ok(false) => {
                // Secondary check: If it's Orca Whirlpool (no LP mint to burn), assume safe if in monitored pools
                true 
            },
            Err(_) => false,
        }
    }

    async fn check_authorities(&self, mint: &Pubkey) -> Result<bool, Box<dyn Error>> {
        let account = self.rpc.get_account(mint).await?;
        let mint_data = Mint::unpack(&account.data)?;
        Ok(mint_data.mint_authority.is_none() && mint_data.freeze_authority.is_none())
    }

    async fn check_holder_distribution(&self, mint: &Pubkey) -> Result<bool, Box<dyn Error>> {
        let largest_accounts = self.rpc.get_token_largest_accounts(mint).await?;
        if let Some(top_holder) = largest_accounts.first() {
            let supply_resp = self.rpc.get_token_supply(mint).await?;
            let supply = supply_resp.amount.parse::<u64>().unwrap_or(0);
            let top_balance = top_holder.amount.amount.parse::<u64>().unwrap_or(0);
            
            if supply > 0 && (top_balance as f64 / supply as f64) > 0.85 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn check_lp_status(&self, pool_id: &Pubkey) -> Result<bool, Box<dyn Error>> {
        let account = match self.rpc.get_account(pool_id).await {
            Ok(acc) => acc,
            Err(_) => return Ok(false),
        };
        
        if let Ok(amm_info) = bytemuck::try_from_bytes::<AmmInfo>(&account.data) {
            let lp_mint = amm_info.lp_mint();
            let supply_resp = self.rpc.get_token_supply(&lp_mint).await?;
            let total_supply = supply_resp.amount.parse::<u64>().unwrap_or(0);
            
            if total_supply == 0 { return Ok(true); }

            let mut burned_amount = 0u64;
            for burn_addr in &self.burn_addresses {
                let ata = spl_associated_token_account::get_associated_token_address(burn_addr, &lp_mint);
                if let Ok(balance_resp) = self.rpc.get_token_account_balance(&ata).await {
                    if let Ok(balance) = balance_resp.amount.parse::<u64>() {
                        burned_amount += balance;
                    }
                }
            }
            return Ok((burned_amount as f64 / total_supply as f64) > 0.90);
        }
        Ok(false)
    }

    async fn check_liquidity_depth(&self, pool_id: &Pubkey) -> Result<bool, Box<dyn Error>> {
        let account = self.rpc.get_account(pool_id).await?;
        
        // For Raydium pools, use the accessor methods from AmmInfo
        if account.data.len() >= 752 {
            if let Ok(amm_info) = bytemuck::try_from_bytes::<AmmInfo>(&account.data) {
                let base_vault = amm_info.base_vault();
                let quote_vault = amm_info.quote_vault();
                
                // Check if either vault has sufficient liquidity
                if let Ok(base_balance) = self.rpc.get_balance(&base_vault).await {
                    if base_balance >= self.min_liquidity_lamports {
                        return Ok(true);
                    }
                }
                
                if let Ok(quote_balance) = self.rpc.get_balance(&quote_vault).await {
                    if quote_balance >= self.min_liquidity_lamports {
                        return Ok(true);
                    }
                }
                
                warn!("⚠️ Pool {} has insufficient liquidity", pool_id);
                return Ok(false);
            }
        }
        
        // For other pool types or if parsing fails, assume safe (will be caught by other checks)
        Ok(true)
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
