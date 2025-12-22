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
    // SubscribeBundleResultsRequest, // Unused for now
    // SendBundleRequest // Unused import in user code sample, checking usage
};
use jito_searcher_client::{get_searcher_client, send_bundle_no_wait};
use tonic::transport::Channel;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::error::Error;
use std::str::FromStr;

pub struct JitoExecutor {
    // The gRPC client must be protected by a Mutex for async sharing
    client: Arc<Mutex<SearcherServiceClient<Channel>>>,
    auth_keypair: Arc<Keypair>,
    rpc_client: Arc<RpcClient>,
    tip_accounts: Vec<Pubkey>,
}

impl JitoExecutor {
    /// Create a new Jito Executor
    /// 
    /// # Arguments
    /// * `block_engine_url` - URL of the Jito Block Engine (e.g., https://amsterdam.mainnet.block-engine.jito.wtf)
    /// * `auth_keypair` - The searcher's keypair for authentication
    /// * `rpc_url` - Generic Solana RPC URL for blockhash fetching
    pub async fn new(block_engine_url: &str, auth_keypair: &Keypair, rpc_url: &str) -> Result<Self, Box<dyn Error>> {
        let auth_arc = Arc::new(Keypair::from_bytes(&auth_keypair.to_bytes())?);
        
        // 1. CONNECT & AUTH (The "Issue" you had is solved here automatically)
        // get_searcher_client performs the Challenge-Response handshake.
        let client = get_searcher_client(block_engine_url, &auth_arc).await?;
        
        let rpc = RpcClient::new(rpc_url.to_string());

        // 2. Fetch Jito Tip Accounts (Hardcoded fallbacks if API fails)
        // These are the official Jito Tip Accounts on Mainnet
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

    /// Construct and Send a Bundle with a Tip
    pub async fn send_bundle(
        &self,
        trade_ixs: Vec<Instruction>,
        tip_amount_lamports: u64,
    ) -> Result<String, Box<dyn Error>> {
        let mut client = self.client.lock().await;
        
        // 1. Get Latest Blockhash
        // We use the standard RPC for this. In production, use geyser-plugin or faster RPC.
        let blockhash = self.rpc_client.get_latest_blockhash()?;

        // 2. Add Tip Instruction (The Bribe)
        // We pick a random tip account to avoid contention
        let tip_account = self.tip_accounts[rand::random::<usize>() % self.tip_accounts.len()];
        let tip_ix = system_instruction::transfer(
            &self.auth_keypair.pubkey(),
            &tip_account,
            tip_amount_lamports
        );

        // 3. Bundle Construction: [Trade, ..., Trade, Tip]
        // Tip MUST be the last instruction or transaction in the bundle usually, 
        // but Jito bundles execute transactions sequentially. 
        // Here we are creating a SINGLE transaction with multiple instructions (Atomicity).
        let mut bundle_ixs = trade_ixs;
        bundle_ixs.push(tip_ix);

        let tx = Transaction::new_signed_with_payer(
            &bundle_ixs,
            Some(&self.auth_keypair.pubkey()),
            &[&*self.auth_keypair], // Sign it
            blockhash,
        );
        
        // Convert to Versioned Transaction (Required by Jito)
        let versioned_tx = VersionedTransaction::from(tx);
        let bundles = vec![versioned_tx];

        // 4. FIRE (Send to Block Engine)
        // send_bundle_no_wait is faster than waiting for response
        send_bundle_no_wait(&mut client, &bundles).await?;

        Ok("Bundle Sent".to_string())
    }
}
