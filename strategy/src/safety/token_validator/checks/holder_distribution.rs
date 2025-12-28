use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::error::Error;

/// Checks if the token has a safe holder distribution.
/// Returns false if the top holder owns more than 85% of the supply.
pub async fn check_holder_distribution(rpc: &RpcClient, mint: &Pubkey) -> Result<bool, Box<dyn Error>> {
    let largest_accounts = rpc.get_token_largest_accounts(mint).await?;
    if let Some(top_holder) = largest_accounts.first() {
        let supply_resp = rpc.get_token_supply(mint).await?;
        let supply = supply_resp.amount.parse::<u64>().unwrap_or(0);
        let top_balance = top_holder.amount.amount.parse::<u64>().unwrap_or(0);
        
        if supply > 0 && (top_balance as f64 / supply as f64) > 0.85 {
            return Ok(false);
        }
    }
    Ok(true)
}