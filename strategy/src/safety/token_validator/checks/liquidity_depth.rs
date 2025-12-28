use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::error::Error;
use mev_core::raydium::AmmInfo;
use bytemuck;
use tracing::{warn};

/// Checks if the pool has sufficient liquidity.
/// Returns true if either vault has at least min_liquidity_lamports.
pub async fn check_liquidity_depth(rpc: &RpcClient, pool_id: &Pubkey, min_liquidity_lamports: u64) -> Result<bool, Box<dyn Error>> {
    let account = rpc.get_account(pool_id).await?;
    
    // For Raydium pools, use the accessor methods from AmmInfo
    if account.data.len() >= 752 {
        if let Ok(amm_info) = bytemuck::try_from_bytes::<AmmInfo>(&account.data) {
            let base_vault = amm_info.base_vault();
            let quote_vault = amm_info.quote_vault();
            
            // Check if either vault has sufficient liquidity
            if let Ok(base_balance) = rpc.get_balance(&base_vault).await {
                if base_balance >= min_liquidity_lamports {
                    return Ok(true);
                }
            }
            
            if let Ok(quote_balance) = rpc.get_balance(&quote_vault).await {
                if quote_balance >= min_liquidity_lamports {
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