use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use solana_sdk::program_pack::Pack;
use std::error::Error;

/// Checks if the token mint has proper authorities.
/// Returns true if both mint_authority and freeze_authority are None (renounced).
pub async fn check_authorities(rpc: &RpcClient, mint: &Pubkey) -> Result<bool, Box<dyn Error>> {
    let account = rpc.get_account(mint).await?;
    let mint_data = Mint::unpack(&account.data)?;
    Ok(mint_data.mint_authority.is_none() && mint_data.freeze_authority.is_none())
}