use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use mev_core::raydium::AmmInfo;
use bytemuck;
use tracing::{warn};

/// Checks if the pool has sufficient liquidity.
pub async fn check_liquidity_depth(rpc: &RpcClient, pool_id: &Pubkey, min_liquidity_lamports: u64) -> Result<bool> {
    let account = rpc.get_account(pool_id).await?;
    check_liquidity_from_data(rpc, &account.data, pool_id, min_liquidity_lamports).await
}

pub async fn check_liquidity_from_data(rpc: &RpcClient, data: &[u8], pool_id: &Pubkey, min_liquidity_lamports: u64) -> Result<bool> {
    // For Raydium pools, use the accessor methods from AmmInfo
    if data.len() >= 752 {
        if let Ok(amm_info) = bytemuck::try_from_bytes::<AmmInfo>(data) {
            let base_vault = amm_info.base_vault();
            let quote_vault = amm_info.quote_vault();
            
            // Batch vault balance check
            let vaults = vec![base_vault, quote_vault];
            if let Ok(balances) = rpc.get_multiple_accounts(&vaults).await {
                for (i, acc_opt) in balances.into_iter().enumerate() {
                    if let Some(acc) = acc_opt {
                        if acc.lamports >= min_liquidity_lamports {
                            return Ok(true);
                        }
                        warn!("⚠️ Pool {} vault {} has insufficient balance: {} < {}", 
                            pool_id, vaults[i], acc.lamports, min_liquidity_lamports);
                    }
                }
            }
            
            warn!("⚠️ Pool {} has insufficient total liquidity depth", pool_id);
            return Ok(false);
        }
    }
    
    // For other pool types (like Pump.fun which has virtual reserves already in the update), assume safe here
    Ok(true)
}