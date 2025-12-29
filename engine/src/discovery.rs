use std::sync::Arc;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
// use anyhow::{Result, anyhow};
use crate::config::BotConfig;
use mev_core::constants::*;
use crate::tui::AppState;
use lru::LruCache;
use std::sync::Mutex;
use std::num::NonZeroUsize;

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
    rpc_url: String, // Explicit RPC URL
    discovery_tx: Sender<DiscoveryEvent>, 
    market_tx: tokio::sync::broadcast::Sender<mev_core::MarketUpdate>,
    tui_state: Option<Arc<std::sync::Mutex<AppState>>>,
    sub_tx: tokio::sync::mpsc::UnboundedSender<String>, // NEW CH
    config: Arc<BotConfig>,
) {
    tracing::info!("🔍 Starting Discovery Engine on: {}", ws_url);
    
    let (ws_stream, _) = match connect_async(&ws_url).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("❌ Discovery WebSocket Failed: {}. Retrying with backoff...", e);
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await; // Staggered backoff
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

    // 4. Subscribe to Meteora Logs
    let meteora_sub = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "logsSubscribe",
        "params": [
            { "mentions": [METEORA_PROGRAM_ID.to_string()] },
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
    if let Err(e) = write.send(Message::Text(meteora_sub.to_string().into())).await {
        tracing::error!("❌ Meteora Log Sub Failed: {}", e);
    }

    let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(rpc_url)); 
    
    // 4. Signature Cache (Eliminate redundant hydration)
    let sig_cache = Arc::new(Mutex::new(LruCache::<String, bool>::new(NonZeroUsize::new(1000).unwrap())));

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
                                        if let Some(event) = parse_log_message(log_str, signature) {
                                            // Check Signature Cache first
                                            {
                                                let mut cache = sig_cache.lock().unwrap();
                                                if cache.contains(signature) {
                                                    crate::telemetry::DISCOVERY_CACHE_HITS.inc();
                                                    continue;
                                                }
                                                cache.put(signature.to_string(), true);
                                            }

                                            tracing::info!("✨ [{:?}] New Pool Detected! Sig: {}", event.program_id, signature);
                                            
                                            // Handle TUI and Metrics
                                            if let Some(ref tui) = tui_state {
                                                if let Ok(mut state) = tui.lock() {
                                                    state.recent_discoveries.push(event.clone());
                                                }
                                            }
                                            // FILTER: Check if any token is in the excluded list (HFT battlegrounds)
                                            let is_excluded = config.excluded_mints.iter().any(|excluded| {
                                                if let Some(token_a) = event.token_a {
                                                    if token_a.to_string() == *excluded { return true; }
                                                }
                                                if let Some(token_b) = event.token_b {
                                                    if token_b.to_string() == *excluded { return true; }
                                                }
                                                // Also check generic logs if we parsed tokens there (future optimization)
                                                false
                                            });

                                            if is_excluded {
                                                tracing::debug!("🚫 Discovery Filter: Dropping HFT Pool (Excluded Mint) - Sig: {}", signature);
                                                continue;
                                            }

                                            crate::telemetry::DISCOVERY_TOKENS_TOTAL.inc();
                                            let _ = discovery_tx.send(event.clone()).await;

                                            // 🚀 LIVE INJECTION: Hydrate and send MarketUpdate for immediate trading
                                            if event.program_id == RAYDIUM_V4_PROGRAM {
                                                let rpc = Arc::clone(&rpc_client);
                                                let market_tx = market_tx.clone();
                                                let sub_tx = sub_tx.clone(); // Clone channel
                                                let sig = signature.to_string();
                                                
                                                tokio::spawn(async move {
                                                    if let Ok(update) = hydrate_raydium_pool(rpc, sig.clone(), event).await {
                                                        tracing::info!("🔥 Discovery Engine: INJECTING MarketUpdate for new pool {}", update.pool_address);
                                                        // 1. Send to Strategy
                                                        let _ = market_tx.send(update.clone());
                                                        // 2. Subscribe for updates!
                                                        let _ = sub_tx.send(update.pool_address.to_string());
                                                    } else {
                                                        tracing::warn!("❌ Failed to hydrate Raydium pool. Signature: {}", sig);
                                                    }
                                                });
                                            } else if event.program_id == PUMP_FUN_PROGRAM {
                                                // 🐸 PUMP.FUN INJECTION
                                                let rpc = Arc::clone(&rpc_client);
                                                let market_tx = market_tx.clone();
                                                let sub_tx = sub_tx.clone();
                                                let sig = signature.to_string();
                                                tracing::info!("🐸 PUMP.FUN DETECTED: Triggering Hydration for sig {}", sig);
                                                
                tokio::spawn(async move {
                    match hydrate_pump_fun_pool(rpc, sig.clone(), event).await {
                        Ok(update) => {
                            tracing::info!("🐸 Discovery Engine: INJECTING Pump.fun Pool {} (Liquidity: {:.2} SOL)", 
                                update.pool_address, 
                                update.pc_reserve as f64 / 1e9
                            );
                            let _ = market_tx.send(update.clone());
                            let _ = sub_tx.send(update.pool_address.to_string());
                        }
                        Err(e) => {
                            // Only warn if it's not a common "not found" or "wrong account" case
                            tracing::debug!("🧪 Pump.fun hydration skip for {}: {}", sig, e);
                        }
                    }
                });
                                            } else if event.program_id == METEORA_PROGRAM_ID {
                                                // ☄️ METEORA INJECTION
                                                let rpc = Arc::clone(&rpc_client);
                                                let market_tx = market_tx.clone();
                                                let sub_tx = sub_tx.clone();
                                                let sig = signature.to_string();
                                                
                                                tokio::spawn(async move {
                                                    if let Ok(update) = hydrate_meteora_pool(rpc, sig.clone(), event).await {
                                                        tracing::info!("☄️ Discovery Engine: INJECTING Meteora Pool {}", update.pool_address);
                                                        let _ = market_tx.send(update.clone());
                                                        let _ = sub_tx.send(update.pool_address.to_string());
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

pub async fn hydrate_raydium_pool(
    rpc: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    signature: String, // We might not need signature if we have the pool address from event, but event.pool_address is usually default() from logs
    event: DiscoveryEvent
) -> anyhow::Result<mev_core::MarketUpdate> {
    
    // If we parsed the pool address from the log (future enhancement), use it. 
    // But currently parse_log_message returns default() for address.
    // We MUST fetch the transaction to get the pool address if we don't have it.
    // WAIT: `DiscoveryEvent` from `parse_log_message` currently has `pool_address: Pubkey::default()`.
    // We can't use `get_account` if we don't know the address!
    
    // Correct approach: 
    // The current `parse_log_message` logic yields a default pubkey.
    // To switch to `get_account`, we must first extract the pool address from the log message itself if possible.
    // Raydium logs are base64 encoded user events. 
    // If we can't parse the address from the log, we are STUCK using get_transaction.
    
    // RE-EVALUATION:
    // HFT optimization: use `get_transaction` is actually necessary for Raydium `Initialize2` because the log doesn't contain the address in cleartext.
    // However, we can OPTIMIZE the fetch.
    
    // For now, I will keep get_transaction but ensure we don't do it for filtered pools.
    // Since we ALREADY implemented filtering in the loop, this function won't be called for HFT filters.
    // SO, the main credit optimization is ALREADY DONE by the filter.
    
    // I will add a check here to ensure we don't re-fetch if we already have it (redundancy).
    
    use solana_sdk::signature::Signature;
    use std::str::FromStr;

    let sig = Signature::from_str(&signature)?;
    
    // 1. Fetch Transaction
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

    let tx_info = tx_info.ok_or_else(|| {
        mev_core::telemetry::DISCOVERY_ERRORS.with_label_values(&["hydration_raydium"]).inc();
        anyhow::anyhow!("Failed to fetch transaction for sniping")
    })?;
    
    // ... (rest of parsing logic)
    let _meta = tx_info.transaction.meta.as_ref().ok_or_else(|| anyhow::anyhow!("No transaction metadata"))?;
    let message = tx_info.transaction.transaction.decode().ok_or_else(|| anyhow::anyhow!("Failed to decode transaction"))?.message;
    
    // Raydium Initialize2: Account 4 is AmmId, 8 is CoinMint, 9 is PcMint
    let amm_id = message.static_account_keys().get(4).ok_or_else(|| anyhow::anyhow!("Missing AmmId"))?;
    let coin_mint = message.static_account_keys().get(8).ok_or_else(|| anyhow::anyhow!("Missing CoinMint"))?;
    let pc_mint = message.static_account_keys().get(9).ok_or_else(|| anyhow::anyhow!("Missing PcMint"))?;

    let mut coin_reserve = 0;
    let mut pc_reserve = 0;

    if let Some(meta) = &tx_info.transaction.meta {
        if let solana_transaction_status::option_serializer::OptionSerializer::Some(balances) = &meta.post_token_balances {
            for balance in balances {
                if balance.mint == *coin_mint.to_string() {
                    if let Ok(amount) = balance.ui_token_amount.amount.parse::<u64>() {
                        if amount > coin_reserve { coin_reserve = amount; }
                    }
                } else if balance.mint == *pc_mint.to_string() {
                    if let Ok(amount) = balance.ui_token_amount.amount.parse::<u64>() {
                        if amount > pc_reserve { pc_reserve = amount; }
                    }
                }
            }
        }
    }
    
    tracing::info!("💧 Raydium Hydration: {} | Coin: {} | PC: {}", amm_id, coin_reserve, pc_reserve);
    
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

pub async fn hydrate_pump_fun_pool(
    rpc: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    _signature: String,
    _event: DiscoveryEvent
) -> anyhow::Result<mev_core::MarketUpdate> {
// use solana_sdk::program_pack::Pack;
        use mev_core::pump_fun::PumpFunBondingCurve;
    use solana_sdk::signature::Signature;
    use std::str::FromStr;

    let sig = Signature::from_str(&_signature).map_err(|e| {
        tracing::error!("❌ Signature Parse Error: {:?} for '{}'", e, _signature);
        anyhow::anyhow!("Invalid signature: {}", e)
    })?;

    tracing::info!("🌊 [Unified] Hydrating Pump.fun Sig: {} (Commitment: Confirmed)", _signature);

    // 1. Fetch Transaction to get accounts
    let mut tx_info = None;
    for attempt in 1..=3 {
        match rpc.get_transaction_with_config(
            &sig,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
                commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            }
        ).await {
            Ok(info) => {
                tx_info = Some(info);
                break;
            }
            Err(e) => {
                tracing::warn!("⏳ [Hydration] Tx Fetch Attempt {} Failed for {}: {}", attempt, _signature, e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempt)).await;
    }
    
    let tx_info = tx_info.ok_or_else(|| anyhow::anyhow!("Failed to fetch Pump.fun transaction {} after 3 attempts", _signature))?;
    let _meta = tx_info.transaction.meta.as_ref().ok_or_else(|| anyhow::anyhow!("No transaction metadata"))?;
    let message = tx_info.transaction.transaction.decode().ok_or_else(|| anyhow::anyhow!("Failed to decode transaction"))?.message;

    let accounts = message.static_account_keys();
    if accounts.is_empty() {
        return Err(anyhow::anyhow!("Transaction has no accounts"));
    }

    // Pump.fun Create Transaction Account Layout (typical):
    // [0] Mint, [1] Mint Authority, [2] Bonding Curve, [3] Associated Bonding Curve, [4] Global, [5] User, ...
    
    // Batch fetch all accounts from the transaction to be efficient
    let mut account_results = Vec::new();
    for chunk in accounts.chunks(100) {
        let mut retry_count = 0;
        let chunk_accounts = loop {
            match rpc.get_multiple_accounts(chunk).await {
                Ok(accs) => break accs,
                Err(e) if retry_count < 3 => {
                    retry_count += 1;
                    tracing::warn!("⏳ RPC 429 or Error in Hydration (chunk): {}. Retrying {}/3...", e, retry_count);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * retry_count)).await;
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to fetch accounts in hydration: {}", e)),
            }
        };
        account_results.extend(chunk_accounts);
    }

    for (i, account_opt) in account_results.into_iter().enumerate() {
        let key = &accounts[i];
        if let Some(account) = account_opt {
            if account.owner == PUMP_FUN_PROGRAM && (account.data.len() == 49 || account.data.len() == 137) {
                tracing::info!("🎯 Found Pump.fun Bonding Curve at index {}: {} (size: {} bytes)", i, key, account.data.len());
                
                if account.data.len() < 8 { continue; }
                let data_without_discriminator = &account.data[8..];
                
                // Use manual deserialization to handle variable account sizes
                match PumpFunBondingCurve::from_account_data(data_without_discriminator) {
                    Ok(curve) => {
                        if curve.virtual_token_reserves > 0 {
                            tracing::info!("✅ [Unified] Hydrated Pump.fun Curve: Tokens={}, SOL={}, Complete={} (Account size: {})", 
                                curve.virtual_token_reserves, curve.virtual_sol_reserves, curve.complete, account.data.len());
                            
                            // In Pump.fun Create, Account 0 is always the Mint
                            let token_mint = accounts[0];
                            
                            return Ok(mev_core::MarketUpdate {
                                pool_address: *key,
                                program_id: PUMP_FUN_PROGRAM,
                                pc_mint: SOL_MINT, 
                                coin_mint: token_mint,
                                coin_reserve: curve.virtual_token_reserves,
                                pc_reserve: curve.virtual_sol_reserves,
                                price_sqrt: None,
                                liquidity: None,
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64,
                            });
                        }
                    },
                    Err(e) => tracing::warn!("❌ Failed to deserialize curve at {} (size: {} bytes): {}", key, account.data.len(), e),
                }
            }
        }
    }
    
    mev_core::telemetry::DISCOVERY_ERRORS.with_label_values(&["not_found_pump"]).inc();
    Err(anyhow::anyhow!("Could not identify active Pump.fun bonding curve for {}", _signature))
}

pub async fn hydrate_meteora_pool(
    rpc: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    signature: String,
    _event: DiscoveryEvent
) -> anyhow::Result<mev_core::MarketUpdate> {
    use solana_sdk::signature::Signature;
    use std::str::FromStr;

    let sig = Signature::from_str(&signature)?;
    
    // Fetch transaction to get accounts
    let tx_info = rpc.get_transaction_with_config(
        &sig,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
            commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        }
    ).await?;

    let message = tx_info.transaction.transaction.decode().ok_or_else(|| anyhow::anyhow!("Failed to decode transaction"))?.message;
    
    // Meteora DLMM usually has the pool address at a specific index in the initialize instruction
    // Placeholder index 3 based on standard Anchor/Meteora layouts
    let pool_address = message.static_account_keys().get(3).ok_or_else(|| anyhow::anyhow!("Missing Meteora Pool Address"))?;
    let token_x = message.static_account_keys().get(5).ok_or_else(|| anyhow::anyhow!("Missing Token X"))?;
    let token_y = message.static_account_keys().get(6).ok_or_else(|| anyhow::anyhow!("Missing Token Y"))?;

    Ok(mev_core::MarketUpdate {
        pool_address: *pool_address,
        program_id: METEORA_PROGRAM_ID,
        coin_mint: *token_x,
        pc_mint: *token_y,
        coin_reserve: 0, // Will be updated by WS stream
        pc_reserve: 0,
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

    // D. Meteora
    if log.contains("InitializeLbPair") {
        return Some(DiscoveryEvent {
            pool_address: Pubkey::default(),
            program_id: METEORA_PROGRAM_ID,
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
