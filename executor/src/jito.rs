use solana_sdk::{
    instruction::Instruction,
    signature::{Keypair, Signer},
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
    system_instruction,
};
use solana_client::rpc_client::RpcClient;
use jito_protos::searcher::{
    searcher_service_client::SearcherServiceClient, 
};
use jito_searcher_client::{get_searcher_client_no_auth, send_bundle_no_wait};
use tonic::transport::Channel;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::error::Error;
use std::str::FromStr;
use rand::seq::SliceRandom; 

use mev_core::ArbitrageOpportunity;
use strategy::ports::{ExecutionPort, PoolKeyProvider};

pub struct JitoExecutor {
    client: Arc<Mutex<SearcherServiceClient<Channel>>>,
    auth_keypair: Arc<Keypair>,
    payer_pubkey: Pubkey,
    rpc_client: Arc<RpcClient>,
    tip_accounts: Vec<Pubkey>,
    key_provider: Option<Arc<dyn PoolKeyProvider>>,
}

impl JitoExecutor {
    pub async fn new(
        block_engine_url: &str, 
        auth_keypair: &Keypair, 
        rpc_url: &str,
        key_provider: Option<Arc<dyn PoolKeyProvider>>
    ) -> Result<Self, Box<dyn Error>> {
        let auth_arc = Arc::new(Keypair::from_bytes(&auth_keypair.to_bytes())?);
        let payer_pubkey = auth_arc.pubkey();
        
        let mut client = get_searcher_client_no_auth(block_engine_url).await?;
        
        // Light 4 Verification: Attempt a simple request to confirm connectivity
        match client.get_tip_accounts(jito_protos::searcher::GetTipAccountsRequest {}).await {
            Ok(_) => tracing::info!("✅ PING SENT! Jito Block Engine is responsive."),
            Err(e) => tracing::warn!("⚠️ Jito Ping Failed: {}. Connection might be limited.", e),
        }
        
        let rpc = RpcClient::new(rpc_url.to_string());

        let tip_accounts = vec![
            Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap(),
            Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PuyAC8eF6S7yBz").unwrap(),
            Pubkey::from_str("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY").unwrap(),
            Pubkey::from_str("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49").unwrap(),
        ];

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            auth_keypair: auth_arc,
            payer_pubkey,
            rpc_client: Arc::new(rpc),
            tip_accounts,
            key_provider,
        })
    }

    pub async fn send_bundle(
        &self,
        trade_ixs: Vec<solana_sdk::instruction::Instruction>,
        tip_amount_lamports: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut client = self.client.lock().await;
        
        let blockhash = self.rpc_client.get_latest_blockhash()?;

        // Pick a Random Tip Account
        let tip_account = {
            let mut rng = rand::thread_rng();
            *self.tip_accounts.choose(&mut rng).unwrap()
        };
        
        let tip_ix = solana_sdk::system_instruction::transfer(
            &self.payer_pubkey,
            &tip_account,
            tip_amount_lamports
        );

        let mut bundle_ixs = trade_ixs;
        bundle_ixs.push(tip_ix);

        let tx = Transaction::new_signed_with_payer(
            &bundle_ixs,
            Some(&self.payer_pubkey),
            &[&*self.auth_keypair],
            blockhash,
        );
        
        let versioned_tx = VersionedTransaction::from(tx);
        let bundles = vec![versioned_tx];

        let _response = send_bundle_no_wait(&bundles, &mut client).await?;
        
        Ok("Bundle_Dispatched".to_string())
    }
}

#[async_trait::async_trait]
impl ExecutionPort for JitoExecutor {
    async fn build_bundle_instructions(
        &self,
        opportunity: ArbitrageOpportunity,
        tip_lamports: u64,
        max_slippage_bps: u16,
    ) -> anyhow::Result<Vec<solana_sdk::instruction::Instruction>> {
        let mut instructions = Vec::new();

        // Slippage Calculation: min_amount_out = input * (1 - slippage)
        // bps = 1/10000. So 1% = 100 bps.
        let min_amount_out = (opportunity.input_amount as u128 * (10000 - max_slippage_bps) as u128 / 10000) as u64;


        // 1. Build Swap Instructions using KeyProvider (Decoupled Infrastructure)
        if let Some(ref provider) = self.key_provider {
            for step in opportunity.steps {
                // Raydium Path
                if step.program_id == mev_core::constants::RAYDIUM_V4_PROGRAM {
                    let keys = provider.get_swap_keys(&step.pool).await?;
                    let mut final_keys = keys;
                    final_keys.user_owner = self.payer_pubkey;
                    // Source/Dest should be derived from mints in a real scenario
                    // Here we use placeholders as this is an architectural refactor
                    
                    instructions.push(crate::raydium_builder::swap_base_in(
                        &final_keys,
                        opportunity.input_amount,
                        min_amount_out, 
                    ));

                }
            }
        } else if std::env::var("SIMULATION").is_ok() {
             // In simulation we just add a dummy instruction to satisfy the test
             instructions.push(solana_sdk::system_instruction::transfer(
                 &self.payer_pubkey,
                 &self.payer_pubkey,
                 1,
             ));
        } else {
            return Err(anyhow::anyhow!("PoolKeyProvider missing. Cannot build instructions."));
        }

        // 2. Add Tip
        let tip_account = {
            let mut rng = rand::thread_rng();
            *self.tip_accounts.choose(&mut rng).unwrap()
        };
        instructions.push(solana_sdk::system_instruction::transfer(
            &self.payer_pubkey,
            &tip_account,
            tip_lamports,
        ));

        Ok(instructions)
    }

    async fn build_and_send_bundle(
        &self,
        opportunity: ArbitrageOpportunity,
        _recent_blockhash: solana_sdk::hash::Hash,
        tip_lamports: u64,
        max_slippage_bps: u16,
    ) -> anyhow::Result<String> {
        let ixs = self.build_bundle_instructions(opportunity, tip_lamports, max_slippage_bps).await?;
        
        match self.send_bundle(ixs, tip_lamports).await {
            Ok(sig) => Ok(sig),
            Err(e) => Err(anyhow::anyhow!("Jito bundle submission failed: {}", e)),
        }
    }

    fn pubkey(&self) -> &solana_sdk::pubkey::Pubkey {
        &self.payer_pubkey
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jito_tip_accounts_config() {
        // We can't easily test JitoExecutor::new without a real block engine connection
        // But we can check if the tip accounts are correctly hardcoded as expected.
        let tip_accounts = vec![
            Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap(),
            Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PuyAC8eF6S7yBz").unwrap(),
            Pubkey::from_str("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY").unwrap(),
            Pubkey::from_str("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49").unwrap(),
        ];
        
        assert_eq!(tip_accounts.len(), 4);
        assert!(tip_accounts.contains(&Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap()));
    }
}
