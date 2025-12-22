use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::error::Error;
use std::str::FromStr;

// Internal dependencies
use mev_core::raydium::AmmInfo; 
use executor::raydium_builder::RaydiumSwapKeys;

pub struct PoolKeyFetcher {
    rpc: RpcClient,
}

impl PoolKeyFetcher {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url.to_string()),
        }
    }

    pub async fn fetch_keys(&self, pool_id: &Pubkey) -> Result<RaydiumSwapKeys, Box<dyn Error>> {
        println!("🔍 Fetching full keys for Pool: {}", pool_id);

        // 1. Fetch Raw Account Data from Solana
        let account = self.rpc.get_account(pool_id)?;
        
        // 2. Deserialization (Zero-Copy)
        // We only care about the prefix that matches AmmInfo
        if account.data.len() < std::mem::size_of::<AmmInfo>() {
            return Err("Account data too small for AmmInfo".into());
        }
        let amm_info: &AmmInfo = bytemuck::try_from_bytes(&account.data[..std::mem::size_of::<AmmInfo>()])
            .map_err(|_| "Failed to cast Raydium data layout")?;

        // 3. Program ID (Standard Raydium V4)
        let program_id = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;

        // 4. Derive Authority (PDA)
        let (authority, _) = Pubkey::find_program_address(
            &[&b"amm authority"[..]], 
            &program_id
        );

        // 5. Construct the Keys Struct
        let keys = RaydiumSwapKeys {
            amm_id: *pool_id,
            amm_authority: authority,
            amm_open_orders: amm_info.open_orders,
            amm_target_orders: amm_info.target_orders,
            amm_coin_vault: amm_info.base_vault,
            amm_pc_vault: amm_info.quote_vault,
            
            serum_program_id: amm_info.market_program_id,
            serum_market: amm_info.market_id,
            
            // PLACEHOLDERS (Needs 2nd fetch for Market Account in Prod)
            serum_bids: Pubkey::new_unique(), 
            serum_asks: Pubkey::new_unique(), 
            serum_event_queue: Pubkey::new_unique(), 
            serum_coin_vault: Pubkey::new_unique(), 
            serum_pc_vault: Pubkey::new_unique(), 
            serum_vault_signer: Pubkey::new_unique(), 
            
            user_source_token_account: Pubkey::default(),
            user_dest_token_account: Pubkey::default(),
            user_owner: Pubkey::default(),
            token_program: Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap(),
        };

        Ok(keys)
    }
}
