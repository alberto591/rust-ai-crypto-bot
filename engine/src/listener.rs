use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use mev_core::MarketUpdate; 

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
    
    // We subscribe to "accountSubscribe" for every pool we care about
    // Note: For production with 100+ pools, use 'programSubscribe' instead.
    for account in accounts {
        let subscribe_msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "accountSubscribe",
            "params": [
                account,
                {
                    "encoding": "jsonParsed", 
                    "commitment": "processed" 
                }
            ]
        });
        write.send(Message::Text(subscribe_msg.to_string())).await.unwrap();
    }

    println!("👂 Listening for price updates...");

    // 2. Process Incoming Messages
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                // Check if this is a notification (not the subscription response)
                if let Some(params) = json.get("params") {
                    if let Some(result) = params.get("result") {
                        if let Some(value) = result.get("value") {
                            // Parse Data
                            // Note: real parsing requires checking offsets (byte_muck)
                            // For this snippet, we assume 'jsonParsed' returns nice fields
                            // or serves as a placeholder for the actual parsing logic implemented in Phase 1.
                            
                            // In a real implementation we would parse Reserves here.
                            // For now, we print to show we received data.
                            // println!("Received Update for Pool: {:?}", value); 
                            
                            // We would construct MarketUpdate and send it:
                            // let update = MarketUpdate { ... };
                            // tx.send(update).await.unwrap();
                        }
                    }
                }
            }
        }
    }
}
