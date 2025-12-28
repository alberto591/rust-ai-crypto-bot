use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use crate::config::BotConfig;
use mev_core::constants::*;
use crate::tui::AppState;

#[derive(Debug, Clone)]
pub struct DiscoveryEvent {
    pub pool_address: Pubkey,
    pub program_id: Pubkey,
    pub token_a: Option<Pubkey>,
    pub token_b: Option<Pubkey>,
    pub timestamp: u64,
}

pub async fn start_discovery(
    ws_url: String, 
    discovery_tx: Sender<DiscoveryEvent>, 
    market_tx: tokio::sync::broadcast::Sender<mev_core::MarketUpdate>,
    tui_state: Option<Arc<std::sync::Mutex<AppState>>>
) {
    tracing::info!("🔍 Starting Discovery Engine on: {}", ws_url);

    let (ws_stream, _) = match connect_async(&ws_url).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("❌ Discovery WebSocket Failed: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    // 1. Subscribe to Raydium Logs
    let raydium_sub = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [RAYDIUM_V4_PROGRAM.to_string()] },
            { "commitment": "processed" }
        ]
    });

    // 2. Subscribe to Pump.fun Logs
    let pump_sub = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [PUMP_FUN_PROGRAM.to_string()] },
            { "commitment": "processed" }
        ]
    });

    // 3. Subscribe to Orca Whirlpool Logs
    let orca_sub = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [ORCA_WHIRLPOOL_PROGRAM.to_string()] },
            { "commitment": "processed" }
        ]
    });

    if let Err(e) = write.send(Message::Text(raydium_sub.to_string().into())).await {
        tracing::error!("❌ Raydium Log Sub Failed: {}", e);
    }
    if let Err(e) = write.send(Message::Text(pump_sub.to_string().into())).await {
        tracing::error!("❌ Pump.fun Log Sub Failed: {}", e);
    }
    if let Err(e) = write.send(Message::Text(orca_sub.to_string().into())).await {
        tracing::error!("❌ Orca Log Sub Failed: {}", e);
    }

    let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(ws_url.replace("ws", "http").replace("8546", "8545"))); // Heuristic for RPC URL

    tracing::info!("👂 Discovery Engine ONLINE. Watching for new pools...");

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    if let Some(params) = json.get("params") {
                        if let Some(result) = params.get("result") {
                            if let Some(value) = result.get("value") {
                                if let Some(logs) = value.get("logs").and_then(|l| l.as_array()) {
                                    let signature = value.get("signature").and_then(|s| s.as_str()).unwrap_or("unknown");
                                    
                                    for log in logs {
                                        let log_str = log.as_str().unwrap_or("");
                                        if let Some(mut event) = parse_log_message(log_str, signature) {
                                            tracing::info!("✨ [{:?}] New Pool Detected! Sig: {}", event.program_id, signature);
                                            
                                            // Handle TUI and Metrics
                                            if let Some(ref tui) = tui_state {
                                                if let Ok(mut state) = tui.lock() {
                                                    state.recent_discoveries.push(event.clone());
                                                }
                                            }
                                            crate::telemetry::DISCOVERY_TOKENS_TOTAL.inc();
                                            let _ = discovery_tx.send(event.clone()).await;

                                            // 🚀 LIVE INJECTION: Hydrate and send MarketUpdate for immediate trading
                                            if event.program_id == RAYDIUM_V4_PROGRAM {
                                                let rpc = Arc::clone(&rpc_client);
                                                let market_tx = market_tx.clone();
                                                let sig = signature.to_string();
                                                
                                                tokio::spawn(async move {
                                                    if let Ok(mut update) = hydrate_raydium_pool(rpc, sig, event).await {
                                                        tracing::info!("🔥 Discovery Engine: INJECTING MarketUpdate for new pool {}", update.pool_address);
                                                        let _ = market_tx.send(update);
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => {
                tracing::warn!("🔍 Discovery WebSocket DISRUPTED.");
                break;
            }
            _ => {}
        }
    }
}

async fn hydrate_raydium_pool(
    rpc: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    signature: String,
    event: DiscoveryEvent
) -> anyhow::Result<mev_core::MarketUpdate> {
    use solana_sdk::signature::Signature;
    use std::str::FromStr;

    let sig = Signature::from_str(&signature)?;
    
    // 1. Fetch Transaction (Try a few times if newly created)
    let mut tx_info = None;
    for _ in 0..3 {
        if let Ok(info) = rpc.get_transaction_with_config(
            &sig,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            }
        ).await {
            tx_info = Some(info);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    let tx_info = tx_info.ok_or_else(|| anyhow::anyhow!("Failed to fetch transaction for sniping"))?;
    
    // 2. Extract Accounts from Initialize2 instruction
    // Raydium Initialize2 accounts: [675k, sysvar_rent, amm_id, amm_authority, amm_open_orders, amm_lp_mint, coin_mint, pc_mint, coin_vault, pc_vault, ...]
    let meta = tx_info.transaction.meta.as_ref().ok_or_else(|| anyhow::anyhow!("No transaction metadata"))?;
    let message = tx_info.transaction.transaction.decode().ok_or_else(|| anyhow::anyhow!("Failed to decode transaction"))?.message;
    
    let amm_id = message.static_account_keys().get(4).ok_or_else(|| anyhow::anyhow!("Missing AmmId"))?;
    let coin_mint = message.static_account_keys().get(8).ok_or_else(|| anyhow::anyhow!("Missing CoinMint"))?;
    let pc_mint = message.static_account_keys().get(9).ok_or_else(|| anyhow::anyhow!("Missing PcMint"))?;

    // 3. Extract initial reserves from log or simulate (For Snipe: 100% of PC balance in vault)
    let coin_reserve = 0; // Will be hydrated by first account update
    let pc_reserve = 0;   // Will be hydrated by first account update
    
    Ok(mev_core::MarketUpdate {
        pool_address: *amm_id,
        program_id: RAYDIUM_V4_PROGRAM,
        coin_mint: *coin_mint,
        pc_mint: *pc_mint,
        coin_reserve,
        pc_reserve,
        price_sqrt: None,
        liquidity: None,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
    })
}

pub fn parse_log_message(log: &str, _signature: &str) -> Option<DiscoveryEvent> {
    // A. Raydium (Standard or Migration)
    if log.contains(RAYDIUM_AMM_LOG_TRIGGER) {
        let is_migration = log.contains("pump"); // Heuristic: Pump migrations often have 'pump' in the log metadata
        
        if is_migration {
            tracing::info!("🚀 PUMP.FUN MIGRATION DETECTED! Preparing for sniping...");
        }

        return Some(DiscoveryEvent {
            pool_address: Pubkey::default(),
            program_id: RAYDIUM_V4_PROGRAM,
            token_a: None,
            token_b: None,
            timestamp: 0,
        });
    }
    
    // B. Pump.fun New Token Create
    if log.contains(PUMP_FUN_LOG_TRIGGER) {
        return Some(DiscoveryEvent {
            pool_address: Pubkey::default(),
            program_id: PUMP_FUN_PROGRAM,
            token_a: None,
            token_b: None,
            timestamp: 0,
        });
    }
    
    // C. Orca
    if log.contains("InitializePool") {
        return Some(DiscoveryEvent {
            pool_address: Pubkey::default(),
            program_id: ORCA_WHIRLPOOL_PROGRAM,
            token_a: None,
            token_b: None,
            timestamp: 0,
        });
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_orca_log() {
        let log = "Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]";
        let log_init = "Program log: Instruction: InitializePool";
        
        let event = parse_log_message(log, "sig123");
        assert!(event.is_none());
        
        let event_init = parse_log_message(log_init, "sig123").expect("Should parse Orca init");
        assert_eq!(event_init.program_id, ORCA_WHIRLPOOL_PROGRAM);
    }

    #[test]
    fn test_parse_raydium_log() {
        let log = "Program log: ray_log: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
        let event = parse_log_message(log, "sig123").expect("Should parse Raydium");
        assert_eq!(event.program_id, RAYDIUM_V4_PROGRAM);
    }
}
