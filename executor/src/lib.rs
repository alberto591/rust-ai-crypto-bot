// Instruction Builders (Ready)
pub mod raydium_builder;  // ✅ Raydium V4 swap factory
pub mod orca_builder;     // ✅ Orca Whirlpool swap (placeholder)

// Executors
pub mod legacy;           // ✅ Standard RPC executor (use today)
// pub mod jito;          // ⏳ Jito bundle executor (waiting on connectivity)



use std::sync::Arc;
use jito_searcher_client::get_searcher_client_auth;
use jito_searcher_client::token_authenticator::ClientInterceptor;
use jito_protos::searcher::searcher_service_client::SearcherServiceClient;
use jito_protos::bundle::Bundle;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::VersionedTransaction;
use solana_sdk::instruction::{Instruction, AccountMeta};
use solana_sdk::message::v0::Message;
use solana_sdk::pubkey::Pubkey;
use tonic::codegen::InterceptedService;
use tonic::transport::Channel;
use tracing::{info, error, debug};
use mev_core::{ArbitrageOpportunity, SwapStep, constants::JITO_TIP_PROGRAM};

pub type SearcherClient = SearcherServiceClient<InterceptedService<Channel, ClientInterceptor>>;

pub struct JitoClient {
    client: Arc<tokio::sync::Mutex<SearcherClient>>,
    pub keypair: Arc<Keypair>,
}

impl JitoClient {
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
    pub async fn new(block_engine_url: &str, keypair: Arc<Keypair>) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Connecting to Jito Block Engine at {}", block_engine_url);
        let client = get_searcher_client_auth(block_engine_url, &keypair).await?;
        Ok(Self {
            client: Arc::new(tokio::sync::Mutex::new(client)),
            keypair,
        })
    }

    pub async fn build_and_send_bundle(
        &self, 
        opportunity: ArbitrageOpportunity,
        recent_blockhash: solana_sdk::hash::Hash,
        tip_lamports: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let instructions = self.build_bundle_instructions(opportunity, tip_lamports).await?;

        // 3. Build Versioned Transaction
        let message = Message::try_compile(
            &self.keypair.pubkey(),
            &instructions,
            &[],
            recent_blockhash,
        )?;
        
        let tx = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(message),
            &[&*self.keypair],
        )?;

        // 4. Dry Run Check
        if std::env::var("DRY_RUN").is_ok() {
            info!("DRY RUN | Bundle built successfully. Transactions: 1 | Instructions: {}", instructions.len());
            return Ok("dry_run_id".to_string());
        }

        // 5. Send Bundle
        self.send_bundle(vec![tx]).await
    }

    pub async fn build_bundle_instructions(
        &self,
        opportunity: ArbitrageOpportunity,
        tip_lamports: u64,
    ) -> Result<Vec<Instruction>, Box<dyn std::error::Error>> {
        let mut instructions = Vec::new();

        // 1. Add Swap Instructions for each step
        for step in opportunity.steps {
            instructions.push(self.build_swap_instruction(step));
        }

        // 2. Add Jito Tip Instruction
        let tip_accounts = self.get_tip_accounts().await?;
        if let Some(tip_account) = tip_accounts.first() {
            let tip_pubkey: Pubkey = tip_account.parse()?;
            debug!("Adding tip of {} lamports to account {}", tip_lamports, tip_pubkey);
            instructions.push(solana_sdk::system_instruction::transfer(
                &self.keypair.pubkey(),
                &tip_pubkey,
                tip_lamports,
            ));
        }

        Ok(instructions)
    }

    fn build_swap_instruction(&self, step: SwapStep) -> Instruction {
        // Dex instruction building (Raydium/Orca Whirlpool)
        // Now uses the correct program ID passed from the strategy engine
        debug!("Building swap instruction for pool: {} on DEX: {}", step.pool, step.program_id);
        
        Instruction {
            program_id: step.program_id,
            accounts: vec![
                AccountMeta::new(self.keypair.pubkey(), true),
                AccountMeta::new(step.input_mint, false),
                AccountMeta::new(step.output_mint, false),
                AccountMeta::new_readonly(step.pool, false),
            ],
            data: vec![], // Encoded swap data for Raydium/Orca would go here
        }
    }

    pub async fn send_bundle(&self, transactions: Vec<VersionedTransaction>) -> Result<String, Box<dyn std::error::Error>> {
        let mut client = self.client.lock().await;
        
        let bundle = Bundle {
            header: None,
            packets: transactions.into_iter().map(|tx| {
                jito_protos::convert::proto_packet_from_versioned_tx(&tx)
            }).collect(),
        };

        let response = client.send_bundle(jito_protos::searcher::SendBundleRequest {
            bundle: Some(bundle),
        }).await?;

        Ok(response.into_inner().uuid)
    }

    pub async fn get_tip_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut client = self.client.lock().await;
        let response = client.get_tip_accounts(jito_protos::searcher::GetTipAccountsRequest {}).await?;
        Ok(response.into_inner().accounts)
    }
}
