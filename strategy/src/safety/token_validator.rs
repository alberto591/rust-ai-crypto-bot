use solana_client::nonblocking::rpc_client::RpcClient; // Use Non-blocking Client
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use solana_sdk::program_pack::Pack;
use std::error::Error;
use std::str::FromStr;
use mev_core::raydium::AmmInfo;

pub struct TokenSafetyChecker {
    rpc: RpcClient,
    // Known "Burn Addresses" where LP tokens are sent to be destroyed
    burn_addresses: Vec<Pubkey>,
}

impl TokenSafetyChecker {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url.to_string()),
            burn_addresses: vec![
                Pubkey::from_str("11111111111111111111111111111111").unwrap(), // System Program
                Pubkey::from_str("Dead111111111111111111111111111111111111").unwrap(), // Burn Address
            ],
        }
    }

    /// 🛡️ The Master Check
    /// Returns Ok(true) ONLY if all checks pass. Fails safe (returns false) on any RPC error.
    pub async fn is_safe_to_trade(&self, mint: &Pubkey, pool_id: &Pubkey) -> bool {
        // Run checks in parallel for speed? For now, sequential is safer to reason about.
        
        // 1. Check Authorities (Mint/Freeze)
        match self.check_authorities(mint).await {
            Ok(true) => (),
            Ok(false) => { println!("⛔ SAFETY: Authorities not revoked."); return false; },
            Err(e) => { println!("⚠️ RPC ERROR (Auth Check): {}", e); return false; }, // Fail Safe
        }

        // 2. Check Top Holders (Concentration)
        match self.check_holder_distribution(mint).await {
            Ok(true) => (),
            Ok(false) => { println!("⛔ SAFETY: Whale concentration too high."); return false; },
            Err(e) => { println!("⚠️ RPC ERROR (Holder Check): {}", e); return false; },
        }

        // 3. Check LP Lock (The Critical Missing Piece)
        match self.check_lp_status(pool_id).await {
            Ok(true) => (),
            Ok(false) => { println!("⛔ SAFETY: LP not locked/burned."); return false; },
            Err(e) => { println!("⚠️ RPC ERROR (LP Check): {}", e); return false; },
        }

        true // All passed
    }

    async fn check_authorities(&self, mint: &Pubkey) -> Result<bool, Box<dyn Error>> {
        let account = self.rpc.get_account(mint).await?;
        let mint_data = Mint::unpack(&account.data)?;
        
        if mint_data.mint_authority.is_some() || mint_data.freeze_authority.is_some() {
            return Ok(false);
        }
        Ok(true)
    }

    async fn check_holder_distribution(&self, mint: &Pubkey) -> Result<bool, Box<dyn Error>> {
        let largest_accounts = self.rpc.get_token_largest_accounts(mint).await?;
        if let Some(top_holder) = largest_accounts.first() {
            let supply_resp = self.rpc.get_token_supply(mint).await?;
            let supply = supply_resp.amount.parse::<u64>()?;
            let top_balance = top_holder.amount.amount.parse::<u64>()?;
            
            // Allow if top holder is < 30%, OR if we can prove it's a locked contract (future upgrade)
            // For now, strict < 80% to filter blatant scams
            if (top_balance as f64 / supply as f64) > 0.80 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn check_lp_status(&self, pool_id: &Pubkey) -> Result<bool, Box<dyn Error>> {
        // 1. Fetch Pool Account
        let account = self.rpc.get_account(pool_id).await?;
        
        // 2. Parse Raydium AmmInfo
        // Raydium V4 AmmInfo is often preceded by a discriminator or padding in some contexts,
        // but the core struct defined in mev-core matches the main data block.
        let amm_info = bytemuck::try_from_bytes::<AmmInfo>(&account.data)
            .map_err(|e| format!("Failed to parse AmmInfo: {}", e))?;
        
        let lp_mint = amm_info.lp_mint;

        // 3. Get total supply
        let supply_resp = self.rpc.get_token_supply(&lp_mint).await?;
        let total_supply = supply_resp.amount.parse::<u64>()?;
        
        if total_supply == 0 {
            return Ok(true); // Should not happen for active pools
        }

        // 4. Check balances in burn addresses
        let mut burned_amount = 0u64;
        for burn_addr in &self.burn_addresses {
            // Find the ATA for this burn address
            let ata = spl_associated_token_account::get_associated_token_address(burn_addr, &lp_mint);
            if let Ok(ata_account) = self.rpc.get_token_account_balance(&ata).await {
                if let Ok(balance) = ata_account.amount.parse::<u64>() {
                    burned_amount += balance;
                }
            }
        }

        // 5. Verify percentage (Safe if > 90% burned)
        let burn_percentage = burned_amount as f64 / total_supply as f64;
        if burn_percentage < 0.90 {
            return Ok(false);
        }

        Ok(true) 
    }
}
