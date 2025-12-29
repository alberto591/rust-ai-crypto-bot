use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use mev_core::raydium::AmmInfo;
use bytemuck;
use spl_associated_token_account;

/// Checks if the liquidity pool has burned LP tokens.
pub async fn check_lp_status(rpc: &RpcClient, pool_id: &Pubkey, burn_addresses: &[Pubkey]) -> Result<bool> {
    let account = match rpc.get_account(pool_id).await {
        Ok(acc) => acc,
        Err(_) => return Ok(false),
    };
    check_lp_status_from_data(rpc, &account.data, pool_id, burn_addresses).await
}

pub async fn check_lp_status_from_data(rpc: &RpcClient, data: &[u8], pool_id: &Pubkey, burn_addresses: &[Pubkey]) -> Result<bool> {
    if let Ok(amm_info) = bytemuck::try_from_bytes::<AmmInfo>(data) {
        let lp_mint = amm_info.lp_mint();
        let supply_resp = rpc.get_token_supply(&lp_mint).await?;
        let total_supply = supply_resp.amount.parse::<u64>().unwrap_or(0);
        
        if total_supply == 0 { return Ok(true); }

        // Batch fetch burn addresses balances
        let atas: Vec<Pubkey> = burn_addresses.iter()
            .map(|ba| spl_associated_token_account::get_associated_token_address(ba, &lp_mint))
            .collect();
        
        let mut burned_amount = 0u64;
        if let Ok(accounts) = rpc.get_multiple_accounts(&atas).await {
            for acc_opt in accounts {
                if let Some(acc) = acc_opt {
                    // This is a bit simplified, ideally should parse TokenAccount
                    // but lamports on an ATA of a burned LP token is a good proxy or we use data.
                    let data = acc.data;
                    if data.len() == 165 {
                        let amount_bytes: [u8; 8] = data[64..72].try_into().unwrap_or([0; 8]);
                        burned_amount += u64::from_le_bytes(amount_bytes);
                    }
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