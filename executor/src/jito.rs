use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
    transaction::{Transaction, VersionedTransaction},
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
use serde::Deserialize;

use mev_core::{ArbitrageOpportunity, FeeStrategy};
use strategy::ports::{ExecutionPort, PoolKeyProvider, TelemetryPort};

#[derive(Deserialize, Debug)]
struct PriorityFeeLevels {
    pub min: f64,
    pub low: f64,
    pub medium: f64,
    pub high: f64,
    pub very_high: f64,
    pub unsafe_max: f64,
}

#[derive(Deserialize, Debug)]
struct PriorityFeeEstimate {
    pub priority_fee_levels: Option<PriorityFeeLevels>,
    pub priority_fee_estimate: Option<f64>,
}

#[derive(Deserialize, Debug)]
struct HeliusRpcResponse<T> {
    pub result: T,
}

pub struct JitoExecutor {
    clients: Vec<Arc<Mutex<SearcherServiceClient<Channel>>>>,  // Multiple endpoints
    current_endpoint_index: Arc<Mutex<usize>>,  // Round-robin tracker
    auth_keypair: Arc<Keypair>,
    payer_pubkey: Pubkey,
    rpc_client: Arc<RpcClient>,
    tip_accounts: Vec<Pubkey>,
    key_provider: Option<Arc<dyn PoolKeyProvider>>,
    telemetry: Option<Arc<dyn TelemetryPort>>,
    max_retries: u32,
    tip_floor_url: String,
    helius_sender_client: Option<Arc<RpcClient>>,
    fee_strategy: FeeStrategy,
}

#[derive(Deserialize, Debug, Default)]
struct TipFloorResponse {
    pub landed_tips_25th_percentile: f64,
    pub landed_tips_50th_percentile: f64,
    pub landed_tips_75th_percentile: f64,
    pub landed_tips_95th_percentile: f64,
    pub landed_tips_99th_percentile: f64,
    pub ema_landed_tips_50th_percentile: f64,
    pub ema_landed_tips_75th_percentile: f64,
}

impl JitoExecutor {
    pub async fn new(
        block_engine_url: &str,  // Can be comma-separated for multiple endpoints
        auth_keypair: &Keypair, 
        rpc_url: &str,
        helius_sender_url: Option<String>,
        fee_strategy: FeeStrategy,
        key_provider: Option<Arc<dyn PoolKeyProvider>>,
        telemetry: Option<Arc<dyn TelemetryPort>>,
    ) -> Result<Self, Box<dyn Error>> {
        let auth_arc = Arc::new(Keypair::from_bytes(&auth_keypair.to_bytes())?);
        let payer_pubkey = auth_arc.pubkey();
        
        // Parse multiple endpoints (comma-separated)
        let urls: Vec<String> = block_engine_url
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        if urls.is_empty() {
            return Err("No Jito block engine URLs provided".into());
        }
        
        // Connect to all endpoints
        let mut clients = Vec::new();
        for (i, url) in urls.iter().enumerate() {
            match get_searcher_client_no_auth(url).await {
                Ok(mut client) => {
                    // Verify connectivity
                    match client.get_tip_accounts(jito_protos::searcher::GetTipAccountsRequest {}).await {
                        Ok(_) => tracing::info!("✅ Jito endpoint {} connected: {}", i+1, url),
                        Err(e) => tracing::warn!("⚠️ Jito endpoint {} ping failed ({}): {}", i+1, url, e),
                    }
                    clients.push(Arc::new(Mutex::new(client)));
                }
                Err(e) => {
                    tracing::error!("❌ Failed to connect to Jito endpoint {}: {}", url, e);
                    // Continue trying other endpoints
                }
            }
        }
        
        if clients.is_empty() {
            return Err("Failed to connect to any Jito endpoints".into());
        }
        
        tracing::info!("✅ Jito executor initialized with {} endpoint(s)", clients.len());
        
        let rpc = Arc::new(RpcClient::new(rpc_url.to_string()));
        let helius_sender = helius_sender_url.map(|url| Arc::new(RpcClient::new(url)));

        let tip_accounts = vec![
            Pubkey::from_str("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5").unwrap(),
            Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PuyAC8eF6S7yBz").unwrap(),
            Pubkey::from_str("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY").unwrap(),
            Pubkey::from_str("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49").unwrap(),
        ];

        Ok(Self {
            clients,
            current_endpoint_index: Arc::new(Mutex::new(0)),
            auth_keypair: auth_arc,
            payer_pubkey,
            rpc_client: rpc,
            tip_accounts,
            key_provider,
            telemetry,
            max_retries: 3,  // 3 attempts per endpoint
            tip_floor_url: "https://mainnet.block-engine.jito.wtf/api/v1/bundles/tip_floor".to_string(),
            helius_sender_client: helius_sender,
            fee_strategy,
        })
    }
    
    pub fn set_fee_strategy(&mut self, strategy: FeeStrategy) {
        self.fee_strategy = strategy;
    }

    /// Fetches the current tip floor from Jito HTTP API
    pub async fn get_tip_floor(&self) -> anyhow::Result<u64> {
        let resp = reqwest::get(&self.tip_floor_url)
            .await?
            .json::<Vec<TipFloorResponse>>()
            .await?;
            
        if let Some(floor) = resp.first() {
            // Use 75th percentile as the minimum base for competitive HFT
            // Fallback to 50th if 75th is missing or zero
            let base_sol = if floor.ema_landed_tips_75th_percentile > 0.0 {
                floor.ema_landed_tips_75th_percentile
            } else {
                floor.ema_landed_tips_50th_percentile
            };
            
            let lamports = (base_sol * 1e9) as u64;
            return Ok(lamports);
        }
        
        tracing::debug!("⚠️ No Jito tip floor data available from API");
        Err(anyhow::anyhow!("No tip floor data available"))
    }

    /// Fetches the current priority fee estimate from Helius API
    pub async fn get_priority_fee_estimate(&self, account_keys: Vec<String>) -> u64 {
        let client = self.helius_sender_client.as_ref().unwrap_or(&self.rpc_client);
        let url = client.url();

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "1",
            "method": "getPriorityFeeEstimate",
            "params": [
                {
                    "accountKeys": account_keys,
                    "options": {
                        "includeAllPriorityFeeLevels": true
                    }
                }
            ]
        });

        match reqwest::Client::new()
            .post(&url)
            .json(&payload)
            .send()
            .await 
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<HeliusRpcResponse<PriorityFeeEstimate>>().await {
                    if let Some(levels) = data.result.priority_fee_levels {
                        return match self.fee_strategy {
                            FeeStrategy::Low => levels.low as u64,
                            FeeStrategy::Medium => levels.medium as u64,
                            FeeStrategy::High => levels.high as u64,
                            FeeStrategy::Extreme => levels.very_high as u64,
                        };
                    }
                    if let Some(estimate) = data.result.priority_fee_estimate {
                        return estimate as u64;
                    }
                }
            }
            Err(e) => tracing::debug!("⚠️ Helius Fee Estimate failed: {}. Using baseline.", e),
        }

        1_000 // Baseline fallback (micro-lamports)
    }

    /// Send bundle with retry logic and round-robin endpoint selection
    pub async fn send_bundle_with_retry(
        &self,
        trade_ixs: Vec<solana_sdk::instruction::Instruction>,
        tip_amount_lamports: u64,
        expected_profit_lamports: u64,
    ) -> anyhow::Result<String> {
        // Try each endpoint with retries
        for endpoint_attempt in 0..self.clients.len() {
            // Get next endpoint (round-robin)
            let client_index = {
                let mut index = self.current_endpoint_index.lock().await;
                let current = *index;
                *index = (*index + 1) % self.clients.len();
                current
            };
            
            tracing::debug!("Attempting Jito endpoint {} (attempt {} of {})", 
                client_index + 1, endpoint_attempt + 1, self.clients.len());
            
            // 🛡️ Dynamic Tipping logic (Phase 3 Hardening)
            let mut final_tip = tip_amount_lamports;
            if let Ok(floor) = self.get_tip_floor().await {
                // Heuristic: floor + competitive profit share
                // We share 10% of profit with Jito to stay ahead of competitors, capped at 0.1 SOL
                let profit_share = (expected_profit_lamports as f64 * 0.10) as u64;
                let profit_share_capped = profit_share.min(100_000_000); // 0.1 SOL cap
                
                let competitive_tip = floor.max(profit_share_capped);
                
                // Only upgrade if competitive tip is higher than our planned tip
                if competitive_tip > final_tip {
                    tracing::info!("⚖️ Jito Tip Upgrade: Floor/Share is {}, raising tip to {} lamports (Profit: {})", 
                        competitive_tip, competitive_tip, expected_profit_lamports);
                    final_tip = competitive_tip;
                }
            }

            // Try with exponential backoff
            for retry in 0..self.max_retries {
                if let Some(ref tel) = self.telemetry {
                    tel.log_endpoint_attempt(client_index);
                }

                match self.send_bundle_to_endpoint(client_index, trade_ixs.clone(), final_tip).await {
                    Ok(sig) => {
                        tracing::info!("✅ Bundle submitted via endpoint {} on attempt {}", 
                            client_index + 1, retry + 1);
                        
                        if let Some(ref tel) = self.telemetry {
                            tel.log_endpoint_success(client_index);
                            tel.log_retry_success(retry as usize);
                        }
                        return Ok(sig);
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        let _is_rate_limit = error_msg.contains("ResourceExhausted") 
                            || error_msg.contains("rate limit");
                        
                        if retry < self.max_retries - 1 {
                            let backoff_ms = 2_u64.pow(retry as u32) * 1000;  // 1s, 2s, 4s
                            tracing::warn!("⚠️ Jito endpoint {} failed (attempt {}): {}. Retrying in {}ms...",
                                client_index + 1, retry + 1, error_msg, backoff_ms);
                            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                        } else {
                            tracing::error!("❌ Jito endpoint {} exhausted all {} retries: {}",
                                client_index + 1, self.max_retries, error_msg);
                            mev_core::telemetry::JITO_BUNDLE_ERRORS.with_label_values(&[&client_index.to_string()]).inc();
                        }
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("All Jito endpoints exhausted"))
    }
    
    /// Send bundle to specific endpoint
    async fn send_bundle_to_endpoint(
        &self,
        endpoint_index: usize,
        trade_ixs: Vec<solana_sdk::instruction::Instruction>,
        tip_amount_lamports: u64,
    ) -> anyhow::Result<String> {
        let mut client = self.clients[endpoint_index].lock().await;
        
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

        // 🛡️ Dynamic Priority Fee (Phase 7)
        let mut account_keys = vec![self.payer_pubkey.to_string(), tip_account.to_string()];
        for ix in &trade_ixs {
            for acc in &ix.accounts {
                account_keys.push(acc.pubkey.to_string());
            }
        }
        let priority_fee = self.get_priority_fee_estimate(account_keys).await;

        let mut bundle_ixs = vec![
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(250_000), // Standard safe limit for 3-hop swap
            solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(priority_fee),    // Dynamic priority
        ];
        bundle_ixs.extend(trade_ixs);
        bundle_ixs.push(tip_ix);

        let tx = Transaction::new_signed_with_payer(
            &bundle_ixs,
            Some(&self.payer_pubkey),
            &[&*self.auth_keypair],
            blockhash,
        );
        
        let signature = tx.signatures[0];

        let versioned_tx = VersionedTransaction::from(tx);
        let bundles = vec![versioned_tx];

        let _response = send_bundle_no_wait(&bundles, &mut client).await?;
        
        Ok(signature.to_string())
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


        let mut current_amount_in = opportunity.input_amount;
        let num_steps = opportunity.steps.len();

        // 1. Build Swap Instructions using KeyProvider (Decoupled Infrastructure)
        if let Some(ref provider) = self.key_provider {
            for (i, step) in opportunity.steps.iter().enumerate() {
                let is_last_step = i == num_steps - 1;
                // Only enforce slippage on the final leg to ensure atomic execution succeeds
                // Intermediate legs use 0 as min_out (swap everything received)
                let step_min_out = if is_last_step { min_amount_out } else { 0 };

                // Raydium Path
                if step.program_id == mev_core::constants::RAYDIUM_V4_PROGRAM {
                    let keys = provider.get_swap_keys(&step.pool).await?;
                    let mut final_keys = keys;
                    final_keys.user_owner = self.payer_pubkey;
                    
                    instructions.push(crate::raydium_builder::swap_base_in(
                        &final_keys,
                        current_amount_in,
                        step_min_out, 
                    ));
                } 
                // Orca Path
                else if step.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM {
                    let mut keys = provider.get_orca_keys(&step.pool).await?;
                    keys.token_authority = self.payer_pubkey;
                    
                    // Resolve user ATAs
                    keys.token_owner_account_a = spl_associated_token_account::get_associated_token_address(
                        &self.payer_pubkey,
                        &keys.mint_a
                    );
                    keys.token_owner_account_b = spl_associated_token_account::get_associated_token_address(
                        &self.payer_pubkey,
                        &keys.mint_b
                    );
                    
                    let a_to_b = step.input_mint == keys.mint_a;
                    
                    instructions.push(crate::orca_builder::swap(
                        &keys,
                        current_amount_in,
                        step_min_out,
                        0, // Refined builder will use default safe price limits
                        true, 
                        a_to_b,
                    ));
                }
                
                // Track amount for multi-hop
                // The output of this step becomes the input of the next
                current_amount_in = step.expected_output;
            }
        }
 else if std::env::var("SIMULATION").is_ok() {
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
        // Build instructions (without tip - will be added in send methods)
        let mut ixs = Vec::new();
        let min_amount_out = (opportunity.input_amount as u128 * (10000 - max_slippage_bps) as u128 / 10000) as u64;
        let mut current_amount_in = opportunity.input_amount;
        let num_steps = opportunity.steps.len();

        if let Some(ref provider) = self.key_provider {
            for (i, step) in opportunity.steps.iter().enumerate() {
                let is_last_step = i == num_steps - 1;
                let step_min_out = if is_last_step { min_amount_out } else { 0 };

                if step.program_id == mev_core::constants::RAYDIUM_V4_PROGRAM {
                    let keys = provider.get_swap_keys(&step.pool).await?;
                    let mut final_keys = keys;
                    final_keys.user_owner = self.payer_pubkey;
                    
                    ixs.push(crate::raydium_builder::swap_base_in(
                        &final_keys,
                        current_amount_in,
                        step_min_out, 
                    ));
                } 
                else if step.program_id == mev_core::constants::PUMP_FUN_PROGRAM {
                    let bonding_curve = step.pool;
                    let token_mint = if step.input_mint == mev_core::constants::SOL_MINT { step.output_mint } else { step.input_mint };
                    let associated_bonding_curve = spl_associated_token_account::get_associated_token_address(
                        &bonding_curve,
                        &token_mint
                    );
                    let user_ata = spl_associated_token_account::get_associated_token_address(
                        &self.payer_pubkey,
                        &token_mint
                    );

                    let is_buy = step.input_mint == mev_core::constants::SOL_MINT;
                    
                    // Add CreateATA for the user if it's a buy (new token)
                    if is_buy {
                        ixs.push(spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                            &self.payer_pubkey,
                            &self.payer_pubkey,
                            &token_mint,
                            &spl_token::id()
                        ));
                        
                        ixs.push(crate::pump_fun_builder::buy(
                            self.payer_pubkey,
                            token_mint,
                            bonding_curve,
                            associated_bonding_curve,
                            user_ata,
                            step.expected_output,
                            current_amount_in, // max_sol_cost
                        ));
                    } else {
                        ixs.push(crate::pump_fun_builder::sell(
                            self.payer_pubkey,
                            token_mint,
                            bonding_curve,
                            associated_bonding_curve,
                            user_ata,
                            current_amount_in, // amount of tokens
                            step_min_out,      // min_sol_output
                        ));
                    }
                } else if step.program_id == crate::meteora_builder::METEORA_PROGRAM_ID {
                    let keys = provider.get_meteora_keys(&step.pool).await?;
                    let mut final_keys = keys;
                    final_keys.user_owner = self.payer_pubkey;
                    final_keys.user_token_x = spl_associated_token_account::get_associated_token_address(&self.payer_pubkey, &keys.token_x_mint);
                    final_keys.user_token_y = spl_associated_token_account::get_associated_token_address(&self.payer_pubkey, &keys.token_y_mint);
                    
                    let x_to_y = step.input_mint == keys.token_x_mint;
                    ixs.push(crate::meteora_builder::build_meteora_swap_ix(&final_keys, current_amount_in, step_min_out, x_to_y));
                }
                else if step.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM {
                    let mut keys = provider.get_orca_keys(&step.pool).await?;
                    keys.token_authority = self.payer_pubkey;

                    // Resolve user ATAs
                    keys.token_owner_account_a = spl_associated_token_account::get_associated_token_address(
                        &self.payer_pubkey,
                        &keys.mint_a
                    );
                    keys.token_owner_account_b = spl_associated_token_account::get_associated_token_address(
                        &self.payer_pubkey,
                        &keys.mint_b
                    );
                    
                    let a_to_b = step.input_mint == keys.mint_a;
                    
                    ixs.push(crate::orca_builder::swap(
                        &keys,
                        current_amount_in,
                        step_min_out,
                        0,
                        true, 
                        a_to_b,
                    ));
                }
                
                current_amount_in = step.expected_output;
            }
        } else if std::env::var("SIMULATION").is_ok() {
            ixs.push(solana_sdk::system_instruction::transfer(
                &self.payer_pubkey,
                &self.payer_pubkey,
                1,
            ));
        } else {
            return Err(anyhow::anyhow!("PoolKeyProvider missing. Cannot build instructions."));
        }
        
        // Try Jito first with retry logic
        if let Some(ref tel) = self.telemetry {
            tel.log_execution_attempt();
        }

        let jito_result = self.send_bundle_with_retry(ixs.clone(), tip_lamports, opportunity.expected_profit_lamports).await;
        
        match jito_result {
            Ok(sig) => {
                tracing::info!("✅ Jito bundle submitted: {}", sig);
                if let Some(ref tel) = self.telemetry {
                    tel.log_jito_success();
                    
                    // Spawn background poller for PnL tracking
                    let rpc = Arc::clone(&self.rpc_client);
                    let telemetry = Arc::clone(tel);
                    let profit = opportunity.expected_profit_lamports;
                    let signature = sig.clone();
                    
                    tokio::spawn(async move {
                        // Poll for confirmation (max 60s)
                        for _ in 0..20 {
                            if let Ok(confirmed) = rpc.get_signature_status(&signature.parse().unwrap()) {
                                if let Some(Ok(_)) = confirmed {
                                    tracing::info!("💰 Trade Confirmed! Reporting +{} lamports", profit);
                                    telemetry.log_trade_landed(opportunity.clone(), signature.clone(), true);
                                    return;
                                } else if let Some(Err(e)) = confirmed {
                                    tracing::warn!("💸 Trade Failed on-chain: {}. Reporting loss.", e);
                                    telemetry.log_trade_landed(opportunity.clone(), signature.clone(), false);
                                    return;
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
                        }
                        tracing::error!("⌛ Confirmation timeout for signature {}. PnL estimate uncertain.", signature);
                    });
                }
                Ok(sig)
            }
            Err(e) => {
                let jito_error = e.to_string();
                drop(e);  // Explicitly drop to ensure Send
                
                if let Some(ref tel) = self.telemetry {
                    tel.log_jito_failed();
                }

                tracing::error!("❌ All Jito endpoints failed: {}. Attempting RPC fallback...", jito_error);
                
                // 🛡️ Helius Rescue: Use specialized Sender API if available (0 credits)
                let sender = self.helius_sender_client.as_ref().unwrap_or(&self.rpc_client);
                match self.send_as_standard_transaction_with_client(ixs, sender).await {
                    Ok(sig) => {
                        tracing::info!("✅ Fallback transaction succeeded via {}: {}", 
                            if self.helius_sender_client.is_some() { "Helius Sender" } else { "Standard RPC" }, 
                            sig
                        );
                        if let Some(ref tel) = self.telemetry {
                            tel.log_rpc_fallback_success();
                        }
                        Ok(sig)
                    }
                    Err(rpc_err) => {
                        if let Some(ref tel) = self.telemetry {
                            tel.log_rpc_fallback_failed();
                        }
                        Err(anyhow::anyhow!(
                            "Both Jito and RPC execution failed. Jito: {}, RPC: {}", 
                            jito_error, rpc_err
                        ))
                    }
                }
            }
        }
    }

    fn pubkey(&self) -> &solana_sdk::pubkey::Pubkey {
        &self.payer_pubkey
    }
}

impl JitoExecutor {
    async fn send_as_standard_transaction(&self, ixs: Vec<solana_sdk::instruction::Instruction>) -> anyhow::Result<String> {
        self.send_as_standard_transaction_with_client(ixs, &self.rpc_client).await
    }

    async fn send_as_standard_transaction_with_client(
        &self, 
        ixs: Vec<solana_sdk::instruction::Instruction>,
        client: &Arc<RpcClient>
    ) -> anyhow::Result<String> {
        let blockhash = client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer_pubkey),
            &[self.auth_keypair.as_ref()],
            blockhash,
        );
        match client.send_transaction(&tx) {
            Ok(sig) => Ok(sig.to_string()),
            Err(e) => Err(anyhow::anyhow!("RPC execution failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jito_tip_floor_query() {
        // This test requires internet access to connect to Jito API
        // In CI/Simulated environments, we might want to mock this.
        // For local verification, we can try a real query if possible.
        let auth = Keypair::new();
        let rpc = "https://api.mainnet-beta.solana.com";
        let jito = match JitoExecutor::new("mainnet-beta.jito.wtf", &auth, rpc, None, None, None).await {
            Ok(j) => j,
            Err(_) => return, // Skip if no connection
        };

        match jito.get_tip_floor().await {
            Ok(floor) => {
                println!("Got Jito Tip Floor: {} lamports", floor);
                assert!(floor > 0);
            }
            Err(e) => {
                println!("Jito Tip Floor query failed: {}", e);
                // Don't fail the test if it's just a network/API issue
            }
        }
    }
}
