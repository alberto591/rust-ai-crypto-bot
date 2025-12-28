use futures_util::{StreamExt, SinkExt};
use tokio::sync::broadcast::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value}; // This line was intended to be kept, the provided snippet was malformed.
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use mev_core::MarketUpdate; 

use std::str::FromStr;

// Map Account -> Token Pair info (Cached)
#[allow(dead_code)]
struct PoolConfig {
    coin_mint: Pubkey,
    pc_mint: Pubkey,
}

pub async fn start_listener(
    ws_url: String, 
    tx: Sender<MarketUpdate>,
    monitored_pools: HashMap<String, (String, String)> // Pool Addr -> (Coin, Pc)
) {
    tracing::info!("📡 Connecting to Solana WebSocket: {}", ws_url);
    
    let (ws_stream, _) = match connect_async(&ws_url).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("❌ WebSocket Connection Failed: {}", e);
            return;
        }
    };
    
    let (mut write, mut read) = ws_stream.split();

    // 1. Subscribe to the specific Raydium Pool Accounts
    let accounts: Vec<&String> = monitored_pools.keys().collect();
    let mut sub_to_pool = HashMap::new();
    let mut pending_subs = HashMap::new(); // Request ID -> Pool Addr
    
    // 0. Subscribe to Slots (Heartbeat)
    let slot_sub_msg = json!({
        "jsonrpc": "2.0",
        "id": 9999,
        "method": "slotSubscribe"
    });
    if let Err(e) = write.send(Message::Text(slot_sub_msg.to_string().into())).await {
        tracing::error!("❌ Slot Subscription failed: {}", e);
    }

    let mut req_id = 1;
    for account in accounts {
        let msg_id = req_id;
        req_id += 1;
        pending_subs.insert(msg_id, account.clone());

        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": msg_id,
            "method": "accountSubscribe",
            "params": [
                account,
                {
                    "encoding": "base64", 
                    "commitment": "processed" 
                }
            ]
        });
        if let Err(e) = write.send(Message::Text(subscribe_msg.to_string().into())).await {
            tracing::error!("❌ Subscription send failed: {}", e);
            return;
        }
    }

    tracing::info!("👂 Listener ACTIVE ({} pools).", monitored_pools.len());

    // 2. Process Incoming Messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Debug: Log that we got *something* (ignore requests/responses with ID)
                if !text.contains("\"id\":") {
                     tracing::debug!("📩 WS Msg ({} chars): {:.100}...", text.len(), text);
                }
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    // A. Handle Subscription Responses
                    if let Some(id_val) = json.get("id").and_then(|v| v.as_u64()) {
                        if let Some(pool_addr) = pending_subs.get(&(id_val as i32)) {
                            if let Some(sub_id) = json.get("result").and_then(|v| v.as_u64()) {
                                sub_to_pool.insert(sub_id, pool_addr.clone());
                                tracing::info!("✅ Subscribed: {} (ID: {})", pool_addr, sub_id);
                             }
                        }
                        continue;
                    }

                    // B. Handle Notifications
                    if let Some(params) = json.get("params") {
                        let sub_id = params.get("subscription").and_then(|v| v.as_u64()).unwrap_or(0);
                        if let Some(pool_addr_str) = sub_to_pool.get(&sub_id) {
                            if let Some(result) = params.get("result") {
                                if let Some(value) = result.get("value") {
                                    if let Some(data_arr) = value.get("data").and_then(|d| d.as_array()) {
                                        if let Some(update_str) = data_arr.first().and_then(|v| v.as_str()) {
                                            use base64::{Engine as _, engine::general_purpose};
                                            if let Ok(bytes) = general_purpose::STANDARD.decode(update_str) {
                                                let pool_addr = Pubkey::from_str(pool_addr_str).unwrap_or_default();
                                                let ts = std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_secs() as i64;

                                                // 1. Identify DEX by data length or owner
                                                if bytes.len() == 653 { // Orca Whirlpool
                                                    let whirlpool: &mev_core::orca::Whirlpool = unsafe {
                                                        &*(bytes.as_ptr() as *const mev_core::orca::Whirlpool)
                                                    };
                                                    let update = MarketUpdate {
                                                        pool_address: pool_addr,
                                                        program_id: mev_core::constants::ORCA_WHIRLPOOL_PROGRAM,
                                                        coin_mint: whirlpool.token_mint_a(),
                                                        pc_mint: whirlpool.token_mint_b(),
                                                        coin_reserve: 0,
                                                        pc_reserve: 0,
                                                        price_sqrt: Some(whirlpool.sqrt_price()),
                                                        liquidity: Some(whirlpool.liquidity()),
                                                        timestamp: ts,
                                                    };
                                                    if tx.send(update).is_err() { break; }
                                                } else if bytes.len() == 752 { // Raydium V4 CPMM
                                                    let amm_info: &mev_core::raydium::AmmInfo = unsafe {
                                                        &*(bytes.as_ptr() as *const mev_core::raydium::AmmInfo)
                                                    };
                                                    let update = MarketUpdate {
                                                        pool_address: pool_addr,
                                                        program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
                                                        coin_mint: amm_info.base_mint(),
                                                        pc_mint: amm_info.quote_mint(),
                                                        coin_reserve: amm_info.base_reserve(),
                                                        pc_reserve: amm_info.quote_reserve(),
                                                        price_sqrt: None,
                                                        liquidity: None,
                                                        timestamp: ts,
                                                    };
                                                    if tx.send(update).is_err() { break; }
                                                } else if bytes.len() == 1544 { // Raydium CLMM
                                                    // TODO: Detailed Raydium CLMM layout. For now, mark as recognized.
                                                    tracing::debug!("Detected Raydium CLMM update (1544 bytes) for pool {}", pool_addr);
                                                } else {
                                                    tracing::trace!("Ignoring unknown account size: {} bytes for pool {}", bytes.len(), pool_addr);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Ping(payload)) => {
                let _ = write.send(Message::Pong(payload)).await;
            }
            Ok(Message::Close(_)) | Err(_) => {
                tracing::warn!("📡 WebSocket Connection DISRUPTED.");
                break;
            }
            _ => {}
        }
    }
}
