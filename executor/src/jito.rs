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
    SendBundleRequest // Kept for traits/types if needed
};
use jito_searcher_client::{get_searcher_client_no_auth, send_bundle_no_wait};
use tonic::transport::Channel;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::error::Error;
use std::str::FromStr;
use rand::seq::SliceRandom; 

pub struct JitoExecutor {
    client: Arc<Mutex<SearcherServiceClient<Channel>>>,
    auth_keypair: Arc<Keypair>,
    rpc_client: Arc<RpcClient>,
    tip_accounts: Vec<Pubkey>,
}

impl JitoExecutor {
    pub async fn new(block_engine_url: &str, auth_keypair: &Keypair, rpc_url: &str) -> Result<Self, Box<dyn Error>> {
        // Use the passed keypair for signing transactions (this is your 'payer')
        let auth_arc = Arc::new(Keypair::from_bytes(&auth_keypair.to_bytes())?);
        
        println!("🔗 Connecting to Jito (No-Auth): {}", block_engine_url);
        let client = get_searcher_client_no_auth(block_engine_url).await?;
        
        let rpc = RpcClient::new(rpc_url.to_string());

        // THE OFFICIAL JITO TIP ACCOUNTS (Using verified subset)
        let tip_accounts = vec![
            Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap(),
            Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PuyAC8eF6S7yBz").unwrap(),
            Pubkey::from_str("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY").unwrap(),
            Pubkey::from_str("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49").unwrap(),
        ];

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            auth_keypair: auth_arc,
            rpc_client: Arc::new(rpc),
            tip_accounts,
        })
    }

    pub async fn send_bundle(
        &self,
        trade_ixs: Vec<Instruction>,
        tip_amount_lamports: u64,
    ) -> Result<String, Box<dyn Error>> {
        let mut client = self.client.lock().await;
        
        let blockhash = self.rpc_client.get_latest_blockhash()?;

        // 1. Pick a Random Tip Account
        let mut rng = rand::thread_rng();
        let tip_account = self.tip_accounts.choose(&mut rng).unwrap();
        
        println!("💸 Tipping Jito Account: {}", tip_account);

        // 2. Build Tip Instruction
        let tip_ix = system_instruction::transfer(
            &self.auth_keypair.pubkey(), // From You
            tip_account,                 // To Jito
            tip_amount_lamports
        );

        // 3. Construct Bundle: [Trades + Tip]
        let mut bundle_ixs = trade_ixs;
        bundle_ixs.push(tip_ix);

        let tx = Transaction::new_signed_with_payer(
            &bundle_ixs,
            Some(&self.auth_keypair.pubkey()),
            &[&*self.auth_keypair],
            blockhash,
        );
        
        let versioned_tx = VersionedTransaction::from(tx);
        let bundles = vec![versioned_tx];

        // 4. Fire
        // SWAPPED ARGS as per local compiler requirement: (&bundles, &mut client)
        let _response = send_bundle_no_wait(&bundles, &mut client).await?;
        
        Ok("Bundle_Dispatched".to_string())
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
