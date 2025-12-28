use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::error::Error;
use std::str::FromStr;

// Internal dependencies
use mev_core::raydium::{AmmInfo, RaydiumSwapKeys}; 

use std::sync::Arc;

pub struct PoolKeyFetcher {
    rpc: Arc<RpcClient>,
}

#[async_trait::async_trait]
impl strategy::ports::PoolKeyProvider for PoolKeyFetcher {
    async fn get_swap_keys(&self, pool_id: &Pubkey) -> Result<RaydiumSwapKeys, anyhow::Error> {
        let keys = self.fetch_raydium_keys(pool_id).await
            .map_err(|e| anyhow::anyhow!("Raydium key fetch error: {}", e))?;
        Ok(keys)
    }

    async fn get_orca_keys(&self, pool_id: &Pubkey) -> Result<mev_core::orca::OrcaSwapKeys, anyhow::Error> {
        let keys = self.fetch_orca_keys(pool_id).await
            .map_err(|e| anyhow::anyhow!("Orca key fetch error: {}", e))?;
        Ok(keys)
    }
}

use mev_core::orca::{Whirlpool, OrcaSwapKeys};

impl PoolKeyFetcher {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: Arc::new(RpcClient::new(rpc_url.to_string())),
        }
    }

    pub async fn fetch_raydium_keys(&self, pool_id: &Pubkey) -> Result<RaydiumSwapKeys, Box<dyn Error>> {
        tracing::debug!("🔍 Fetching Raydium keys for Pool: {}", pool_id);
        // ... (rest of the existing fetch_keys logic)
        let account = self.rpc.get_account(pool_id)?;
        if account.data.len() < 752 {
            return Err("Account data too small for Raydium V4 (expected 752)".into());
        }
        let amm_info: &AmmInfo = bytemuck::try_from_bytes(&account.data[..752])
            .map_err(|_| "Failed to cast Raydium data layout")?;

        let program_id = mev_core::constants::RAYDIUM_V4_PROGRAM;
        let (authority, _) = Pubkey::find_program_address(&[&b"amm authority"[..]], &program_id);

        // Fetch Serum Market account to get Bids, Asks, Event Queue, and Vaults
        let market_id = amm_info.market_id();
        let market_account = self.rpc.get_account(&market_id)?;
        if market_account.data.len() < 388 {
            return Err("Serum market account data too small".into());
        }
        let market_state: &mev_core::raydium::MarketStateV3 = bytemuck::try_from_bytes(&market_account.data[..388])
            .map_err(|_| "Failed to cast Serum market data layout")?;

        let serum_program_id = amm_info.market_program_id();
        let vault_signer = Pubkey::create_program_address(
            &[
                &market_id.to_bytes(),
                &u64::from(market_state.vault_signer_nonce()).to_le_bytes(),
            ],
            &serum_program_id,
        ).map_err(|_| "Failed to derive Serum vault signer")?;

        Ok(RaydiumSwapKeys {
            amm_id: *pool_id,
            amm_authority: authority,
            amm_open_orders: amm_info.open_orders(),
            amm_target_orders: amm_info.target_orders(),
            amm_coin_vault: amm_info.base_vault(),
            amm_pc_vault: amm_info.quote_vault(),
            serum_program_id,
            serum_market: market_id,
            serum_bids: market_state.bids(),
            serum_asks: market_state.asks(),
            serum_event_queue: market_state.event_queue(),
            serum_coin_vault: market_state.coin_vault(),
            serum_pc_vault: market_state.pc_vault(),
            serum_vault_signer: vault_signer,
            user_source_token_account: Pubkey::default(),
            user_dest_token_account: Pubkey::default(),
            user_owner: Pubkey::default(),
            token_program: mev_core::constants::TOKEN_PROGRAM_ID,
        })
    }

    pub async fn fetch_orca_keys(&self, pool_id: &Pubkey) -> Result<OrcaSwapKeys, Box<dyn Error>> {
        tracing::debug!("🔍 Fetching Orca keys for Pool: {}", pool_id);
        let account = self.rpc.get_account(pool_id)?;
        
        if account.data.len() < 653 {
            return Err("Account data too small for Whirlpool (expected 653)".into());
        }
        
        let whirlpool: &Whirlpool = bytemuck::try_from_bytes(&account.data[..653])
            .map_err(|_| "Failed to cast Orca data layout")?;

        let tick_spacing = whirlpool.tick_spacing();
        let current_tick = whirlpool.tick_current_index();
        let program_id = mev_core::constants::ORCA_WHIRLPOOL_PROGRAM;

        // Derive Tick Arrays (Current, Previous, Next)
        let start_index_0 = OrcaSwapKeys::get_tick_array_start_index(current_tick, tick_spacing);
        let ticks_in_array = OrcaSwapKeys::TICKS_PER_ARRAY * tick_spacing as i32;
        
        let tick_array_0 = OrcaSwapKeys::derive_tick_array_pda(pool_id, start_index_0, &program_id);
        let tick_array_1 = OrcaSwapKeys::derive_tick_array_pda(pool_id, start_index_0 - ticks_in_array, &program_id);
        let tick_array_2 = OrcaSwapKeys::derive_tick_array_pda(pool_id, start_index_0 + ticks_in_array, &program_id);

        // Derive Oracle PDA
        let (oracle, _) = Pubkey::find_program_address(
            &[b"oracle", pool_id.as_ref()],
            &program_id
        );

        Ok(OrcaSwapKeys {
            whirlpool: *pool_id,
            mint_a: whirlpool.token_mint_a(),
            mint_b: whirlpool.token_mint_b(),
            token_authority: Pubkey::default(), // Will be set by executor to payer
            token_owner_account_a: Pubkey::default(), // Will be set by executor
            token_vault_a: whirlpool.token_vault_a(),
            token_owner_account_b: Pubkey::default(), // Will be set by executor
            token_vault_b: whirlpool.token_vault_b(),
            tick_array_0,
            tick_array_1,
            tick_array_2,
            oracle,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raydium_authority_derivation() {
        // Standard Raydium V4 Program ID
        let program_id = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
        
        // Derive Authority (PDA)
        let (authority, _) = Pubkey::find_program_address(
            &[&b"amm authority"[..]], 
            &program_id
        );

        // This is the value produced by find_program_address with [b"amm authority"]
        let expected_authority = Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap();
        assert_eq!(authority, expected_authority);
    }
}
