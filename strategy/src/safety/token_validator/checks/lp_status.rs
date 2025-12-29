use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use mev_core::raydium::AmmInfo;
use bytemuck;
use spl_associated_token_account;

/// Checks if the liquidity pool has burned LP tokens.
/// Returns true if more than 90% of LP tokens are burned.
pub async fn check_lp_status(rpc: &RpcClient, pool_id: &Pubkey, burn_addresses: &[Pubkey]) -> Result<bool> {
    let account = match rpc.get_account(pool_id).await {
        Ok(acc) => acc,
        Err(_) => return Ok(false),
    };
    
    if let Ok(amm_info) = bytemuck::try_from_bytes::<AmmInfo>(&account.data) {
        let _pool = Pubkey::new_unique();
        let lp_mint = amm_info.lp_mint();
        let supply_resp = rpc.get_token_supply(&lp_mint).await?;
        let total_supply = supply_resp.amount.parse::<u64>().unwrap_or(0);
        
        if total_supply == 0 { return Ok(true); }

        let mut burned_amount = 0u64;
        for burn_addr in burn_addresses {
            let ata = spl_associated_token_account::get_associated_token_address(burn_addr, &lp_mint);
            if let Ok(balance_resp) = rpc.get_token_account_balance(&ata).await {
                if let Ok(balance) = balance_resp.amount.parse::<u64>() {
                    burned_amount += balance;
                }
            }
        }
        
        let burn_percentage = burned_amount as f64 / total_supply as f64;
        if burn_percentage <= 0.90 {
            tracing::warn!("⚠️ LP Status failure for pool {}: only {:.2}% burned ({} / {})", pool_id, burn_percentage * 100.0, burned_amount, total_supply);
            return Ok(false);
        }
        return Ok(true);
    }
    tracing::warn!("⚠️ Could not parse AmmInfo for pool {} to check LP status", pool_id);
    Ok(false)
}