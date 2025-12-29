use std::sync::Arc;
use std::collections::HashMap;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::{mpsc, broadcast};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use crate::tui::AppState;
use mev_core::constants::*;
use mev_core::MarketUpdate;
use crate::discovery::{DiscoveryEvent, parse_log_message};
// use mev_core::telemetry::*;
use crate::scoring::PoolScoringEngine;
pub async fn start_market_watcher(
    ws_url: String,
    rpc_url: String,
    discovery_tx: mpsc::Sender<DiscoveryEvent>,
    market_tx: broadcast::Sender<MarketUpdate>,
    tui_state: Option<Arc<std::sync::Mutex<AppState>>>,
    monitored_pools: HashMap<String, (String, String)>,
    mut subscription_rx: mpsc::UnboundedReceiver<String>,
    scoring_engine: Arc<PoolScoringEngine>,
) {
    tracing::info!("📡 Starting Unified MarketWatcher: {}", ws_url);
    let hydration_limit = Arc::new(tokio::sync::Semaphore::new(3)); // Max 3 concurrent GET_TRANSACTION calls

    let mut retry_delay = 2; // Start with 2s
    let mut seen_signatures = std::collections::HashSet::new();
    let mut seen_pools: std::collections::HashMap<String, std::time::Instant> = std::collections::HashMap::new();
    let mut last_cleanup = std::time::Instant::now();
    let mut last_decay = std::time::Instant::now();

    loop {
        // Periodic cleanup of seen signatures and pools (every 5 minutes)
        if last_cleanup.elapsed() > std::time::Duration::from_secs(300) {
            seen_signatures.clear();
            seen_pools.clear();
            last_cleanup = std::time::Instant::now();
        }

        // Periodic weight decay (every 60 seconds)
        if last_decay.elapsed() > std::time::Duration::from_secs(60) {
            scoring_engine.decay_weights();
            last_decay = std::time::Instant::now();
        }

        let (ws_stream, _) = match connect_async(&ws_url).await {
            Ok(s) => {
                retry_delay = 2; // Reset on success
                s
            },
            Err(e) => {
                let jitter = rand::random::<u64>() % 1000;
                tracing::error!("❌ Watcher WebSocket Failed: {}. Retrying in {}s...", e, retry_delay);
                tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay * 1000 + jitter)).await;
                retry_delay = (retry_delay * 2).min(60); // Max 60s
                continue;
            }
        };

        let (mut write, mut read) = ws_stream.split();
        let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(rpc_url.clone()));

        // 1. Initial Subscriptions
        let sub_messages = vec![
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "logsSubscribe",
                "params": [{ "mentions": [RAYDIUM_V4_PROGRAM.to_string()] }, { "commitment": "processed" }]
            }),
            json!({
                "jsonrpc": "2.0", "id": 2, "method": "logsSubscribe",
                "params": [{ "mentions": [PUMP_FUN_PROGRAM.to_string()] }, { "commitment": "processed" }]
            }),
            json!({
                "jsonrpc": "2.0", "id": 3, "method": "logsSubscribe",
                "params": [{ "mentions": [ORCA_WHIRLPOOL_PROGRAM.to_string()] }, { "commitment": "processed" }]
            }),
            json!({
                "jsonrpc": "2.0", "id": 4, "method": "logsSubscribe",
                "params": [{ "mentions": [METEORA_PROGRAM_ID.to_string()] }, { "commitment": "processed" }]
            }),
            json!({
                "jsonrpc": "2.0", "id": 5, "method": "slotSubscribe"
            }),
        ];

        for sub in sub_messages {
            let _ = write.send(Message::Text(sub.to_string().into())).await;
        }

        let mut sub_to_pool = HashMap::new();
        let mut pending_subs = HashMap::new(); // Request ID -> Pool Addr
        let mut req_id = 100;

        for pool_addr in monitored_pools.keys() {
            let mid = req_id; req_id += 1;
            pending_subs.insert(mid, pool_addr.clone());
            let sub_msg = json!({
                "jsonrpc": "2.0", "id": mid, "method": "accountSubscribe",
                "params": [pool_addr, { "encoding": "base64", "commitment": "processed" }]
            });
            let _ = write.send(Message::Text(sub_msg.to_string().into())).await;
        }

        tracing::info!("👂 Unified Watcher ONLINE. Monitoring {} pools + New Discovery.", monitored_pools.len());

        loop {
            tokio::select! {
                Some(new_pool) = subscription_rx.recv() => {
                    let mid = req_id; req_id += 1;
                    pending_subs.insert(mid, new_pool.clone());
                    let sub_msg = json!({
                        "jsonrpc": "2.0", "id": mid, "method": "accountSubscribe",
                        "params": [new_pool, { "encoding": "base64", "commitment": "processed" }]
                    });
                    if let Err(e) = write.send(Message::Text(sub_msg.to_string().into())).await {
                        tracing::error!("❌ Failed dynamic sub send for {}: {}", new_pool, e);
                    }
                }

                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(json) = serde_json::from_str::<Value>(&text) {
                                if let Some(id_val) = json.get("id").and_then(|v| v.as_u64()) {
                                    if let Some(pool_addr) = pending_subs.get(&(id_val as i32)) {
                                        if let Some(sub_id) = json.get("result").and_then(|v| v.as_u64()) {
                                            sub_to_pool.insert(sub_id, pool_addr.clone());
                                            tracing::info!("✅ [Unified] Subscribed: {} (ID: {})", pool_addr, sub_id);
                                        }
                                    }
                                    continue;
                                }

                                if let Some(params) = json.get("params") {
                                    let method = json.get("method").and_then(|m| m.as_str()).unwrap_or("");
                                    let sub_id = params.get("subscription").and_then(|v| v.as_u64()).unwrap_or(0);

                                    match method {
                                        "logsNotification" => {
                                             if let Some(result) = params.get("result") {
                                                if let Some(value) = result.get("value") {
                                                    if let Some(logs) = value.get("logs").and_then(|l| l.as_array()) {
                                                        let signature = value.get("signature").and_then(|s| s.as_str()).unwrap_or("unknown");
                                                        for log in logs {
                                                            let log_str = log.as_str().unwrap_or("");
                                                            if let Some(event) = parse_log_message(log_str, signature) {
                                                                if seen_signatures.insert(signature.to_string()) {
                                                                    let pool_key = event.pool_address.to_string();
                                                                    let should_process = if let Some(last_seen) = seen_pools.get(&pool_key) {
                                                                        if last_seen.elapsed() < std::time::Duration::from_secs(300) {
                                                                            tracing::debug!("⏭️ Skipping duplicate pool: {} (seen {} seconds ago)", pool_key, last_seen.elapsed().as_secs());
                                                                            mev_core::telemetry::POOL_DEDUP_SKIPS.inc();
                                                                            false
                                                                        } else {
                                                                            true
                                                                        }
                                                                    } else {
                                                                        true
                                                                    };
                                                                    
                                                                    if should_process {
                                                                        seen_pools.insert(pool_key, std::time::Instant::now());
                                                                        handle_discovery_event(event, signature, &rpc_client, &market_tx, &discovery_tx, &tui_state, hydration_limit.clone(), Arc::clone(&scoring_engine)).await;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                             }
                                        },
                                        "accountNotification" => {
                                            if let Some(pool_addr_str) = sub_to_pool.get(&sub_id) {
                                                if let Some(result) = params.get("result") {
                                                    if let Some(value) = result.get("value") {
                                                        if let Some(data_arr) = value.get("data").and_then(|d| d.as_array()) {
                                                            if let Some(update_str) = data_arr.first().and_then(|v| v.as_str()) {
                                                                handle_account_update(pool_addr_str, update_str, &market_tx, Arc::clone(&scoring_engine)).await;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        "slotNotification" => {},
                                        _ => {}
                                    }
                                }
                            }
                        },
                        Some(Ok(Message::Ping(payload))) => { let _ = write.send(Message::Pong(payload)).await; },
                        Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                            tracing::warn!("📡 Unified Watcher DISRUPTED. Reconnecting...");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

async fn handle_discovery_event(
    event: DiscoveryEvent,
    signature: &str,
    rpc: &Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    market_tx: &broadcast::Sender<MarketUpdate>,
    discovery_tx: &mpsc::Sender<DiscoveryEvent>,
    tui: &Option<Arc<std::sync::Mutex<AppState>>>,
    semaphore: Arc<tokio::sync::Semaphore>,
    scoring_engine: Arc<PoolScoringEngine>,
) {
    tracing::info!("✨ [{:?}] New Pool Detected! Sig: {}", event.program_id, signature);
    
    if let Some(ref tui) = tui {
        if let Ok(mut state) = tui.lock() {
            state.recent_discoveries.push(event.clone());
        }
    }
    mev_core::telemetry::DISCOVERY_TOKENS_TOTAL.inc();
    let _ = discovery_tx.send(event.clone()).await;

    // Initialize pool weight in scoring engine
    scoring_engine.update_activity(event.pool_address);

    let rpc_clone = Arc::clone(rpc);
    let market_tx_clone = market_tx.clone();
    let sig = signature.to_string();
    let ev = event.clone();
    let sem = semaphore.clone();

    if let Ok(_permit) = sem.clone().try_acquire_owned() {
        tokio::spawn(async move {
            let _permit = _permit;
            if ev.program_id == RAYDIUM_V4_PROGRAM {
                if let Ok(update) = crate::discovery::hydrate_raydium_pool(rpc_clone, sig.clone(), ev).await {
                    tracing::info!("🔥 [Unified] INJECTING Raydium {} for Snipe", update.pool_address);
                    let _ = market_tx_clone.send(update);
                }
            } else if ev.program_id == PUMP_FUN_PROGRAM {
                if let Ok(update) = crate::discovery::hydrate_pump_fun_pool(rpc_clone, sig.clone(), ev).await {
                    tracing::info!("🐸 [Unified] INJECTING Pump.fun {} for Snipe", update.pool_address);
                    let _ = market_tx_clone.send(update);
                }
            } else if ev.program_id == METEORA_PROGRAM_ID {
                if let Ok(update) = crate::discovery::hydrate_meteora_pool(rpc_clone, sig.clone(), ev).await {
                    tracing::info!("☄️ [Unified] INJECTING Meteora {} for Snipe", update.pool_address);
                    let _ = market_tx_clone.send(update);
                }
            }
        });
    } else {
        tracing::debug!("⏳ Hydration throttled (Signature: {})", signature);
    }
}

async fn handle_account_update(pool_addr: &str, data_base64: &str, tx: &broadcast::Sender<MarketUpdate>, scoring_engine: Arc<PoolScoringEngine>) {
    use base64::{Engine as _, engine::general_purpose};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    if let Ok(bytes) = general_purpose::STANDARD.decode(data_base64) {
        let pool_pub = Pubkey::from_str(pool_addr).unwrap_or_default();
        
        // Update pool weight (Activity Bonus)
        scoring_engine.update_activity(pool_pub);

        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        
        if bytes.len() == 653 { // Orca
            let whirlpool: &mev_core::orca::Whirlpool = unsafe { &*(bytes.as_ptr() as *const mev_core::orca::Whirlpool) };
            let _ = tx.send(MarketUpdate {
                pool_address: pool_pub, program_id: ORCA_WHIRLPOOL_PROGRAM,
                coin_mint: whirlpool.token_mint_a(), pc_mint: whirlpool.token_mint_b(),
                coin_reserve: 0, pc_reserve: 0, price_sqrt: Some(whirlpool.sqrt_price()), liquidity: Some(whirlpool.liquidity()),
                timestamp: ts,
            });
        } else if bytes.len() == 752 { // Raydium
            let amm: &mev_core::raydium::AmmInfo = unsafe { &*(bytes.as_ptr() as *const mev_core::raydium::AmmInfo) };
            let _ = tx.send(MarketUpdate {
                pool_address: pool_pub, program_id: RAYDIUM_V4_PROGRAM,
                coin_mint: amm.base_mint(), pc_mint: amm.quote_mint(),
                coin_reserve: amm.base_reserve(), pc_reserve: amm.quote_reserve(),
                price_sqrt: None, liquidity: None, timestamp: ts,
            });
        }
    }
}
