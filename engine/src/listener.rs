use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
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
    println!("📡 Connecting to Solana WebSocket: {}", ws_url);
    
    let (ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();

    // 1. Subscribe to the specific Raydium Pool Accounts
    let accounts: Vec<&String> = monitored_pools.keys().collect();
    let mut sub_to_pool = HashMap::new();
    let mut pending_subs = HashMap::new(); // Request ID -> Pool Addr
    
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
        write.send(Message::Text(subscribe_msg.to_string().into())).await.unwrap();
    }

    println!("👂 Listening for price updates...");

    // 2. Process Incoming Messages
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    // A. Handle Subscription Responses
                    if let Some(id_val) = json.get("id").and_then(|v| v.as_u64()) {
                        if let Some(pool_addr) = pending_subs.get(&(id_val as i32)) {
                            if let Some(sub_id) = json.get("result").and_then(|v| v.as_u64()) {
                                sub_to_pool.insert(sub_id, pool_addr.clone());
                                println!("✅ Subscribed to pool: {} (SubID: {})", pool_addr, sub_id);
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
                                        if let Some(base64_str) = data_arr.get(0).and_then(|s| s.as_str()) {
                                            // Use modern base64 API
                                            use base64::{Engine as _, engine::general_purpose};
                                            if let Ok(bytes) = general_purpose::STANDARD.decode(base64_str) {
                                                if bytes.len() >= std::mem::size_of::<mev_core::raydium::AmmInfo>() {
                                                    let amm_info: &mev_core::raydium::AmmInfo = unsafe {
                                                        &*(bytes.as_ptr() as *const mev_core::raydium::AmmInfo)
                                                    };

                                                    let pool_addr = Pubkey::from_str(pool_addr_str).unwrap_or_default();
                                                    let ts = std::time::SystemTime::now()
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap()
                                                        .as_secs() as i64;
                                                    
                                                    let update = MarketUpdate {
                                                        pool_address: pool_addr,
                                                        coin_mint: amm_info.base_mint,
                                                        pc_mint: amm_info.quote_mint,
                                                        coin_reserve: amm_info.base_reserve,
                                                        pc_reserve: amm_info.quote_reserve,
                                                        timestamp: ts,
                                                    };
                                                    
                                                    if let Err(_) = tx.send(update).await {
                                                        break; 
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
            }
            Ok(Message::Ping(payload)) => {
                let _ = write.send(Message::Pong(payload)).await;
            }
            Ok(Message::Close(_)) | Err(_) => {
                println!("📡 WebSocket Closed by server or error.");
                break;
            }
            _ => {}
        }
    }
}
