use reqwest::Client;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use crate::metrics::BotMetrics;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::time::{Instant, Duration}; // Use tokio's Instant and Duration for async contexts
use serde_json::{json, Value}; // Add Value for parsing Telegram responses
use solana_sdk::pubkey::Pubkey;
use crate::wallet_manager::WalletManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Success,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn to_color(&self) -> u32 {
        match self {
            AlertSeverity::Info => 0x3498DB,     // Blue
            AlertSeverity::Success => 0x2ECC71,  // Green
            AlertSeverity::Warning => 0xF1C40F,  // Yellow/Orange
            AlertSeverity::Critical => 0xE74C3C, // Red
        }
    }

    pub fn to_emoji(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Success => "✅",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Critical => "🚨",
        }
    }
}

pub struct Field {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

pub struct AlertManager {
    discord_webhook: Option<String>,
    telegram_config: Option<TelegramConfig>,
    client: Client,
    last_alerts: Mutex<HashMap<String, Instant>>,
}

pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

impl AlertManager {
    pub fn new(discord_webhook: Option<String>, telegram_config: Option<TelegramConfig>) -> Self {
        Self {
            discord_webhook,
            telegram_config,
            client: Client::new(),
            last_alerts: Mutex::new(HashMap::new()),
        }
    }
    
    pub async fn send_alert(&self, severity: AlertSeverity, title: &str, message: &str, fields: Vec<Field>) {
        // Simple Rate Limiting (Prevent spamming the same title/message within 5 minutes)
        let alert_key = format!("{}:{}", title, message);
        {
            let mut last_alerts = self.last_alerts.lock().await;
            if let Some(last_sent) = last_alerts.get(&alert_key) {
                if last_sent.elapsed() < Duration::from_secs(300) {
                    return; // Skip if sent less than 5 mins ago
                }
            }
            last_alerts.insert(alert_key, Instant::now());
        }

        let emoji = severity.to_emoji();
        let full_title = format!("{} {}", emoji, title);
        let color = severity.to_color();
        self.dispatch_alert(severity, &full_title, message, fields, color).await;
    }

    pub async fn send_critical(&self, message: &str) {
        self.send_alert(AlertSeverity::Critical, "CRITICAL", message, vec![]).await;
    }
    
    pub async fn send_warning(&self, message: &str) {
        self.send_alert(AlertSeverity::Warning, "WARNING", message, vec![]).await;
    }
    
    pub async fn send_success(&self, message: &str) {
        self.send_alert(AlertSeverity::Success, "SUCCESS", message, vec![]).await;
    }
    
    async fn dispatch_alert(&self, _severity: AlertSeverity, title: &str, message: &str, fields: Vec<Field>, color: u32) {
        // Discord webhook
        if let Some(webhook_url) = &self.discord_webhook {
            let mut embed = json!({
                "title": title,
                "description": message,
                "color": color,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if !fields.is_empty() {
                let discord_fields: Vec<_> = fields.iter().map(|f| json!({
                    "name": &f.name,
                    "value": &f.value,
                    "inline": f.inline
                })).collect();
                embed["fields"] = json!(discord_fields);
            }

            let payload = json!({
                "embeds": [embed]
            });
            
            if let Err(e) = self.client.post(webhook_url).json(&payload).send().await {
                tracing::error!("Failed to send Discord alert: {}", e);
            } else {
                tracing::info!("✅ Discord alert dispatched successfully.");
            }
        }
        
        // Telegram
        if let Some(config) = &self.telegram_config {
            let url = format!(
                "https://api.telegram.org/bot{}/sendMessage",
                config.bot_token
            );
            
            let mut text = format!("<b>{}</b>\n\n{}", title, message);
            if !fields.is_empty() {
                for field in &fields {
                    text.push_str(&format!("\n\n<b>{}</b>: {}", field.name, field.value));
                }
            }

            let payload = json!({
                "chat_id": config.chat_id,
                "text": text,
                "parse_mode": "HTML",
            });
            
            match self.client.post(&url).json(&payload).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let err_text = resp.text().await.unwrap_or_default();
                        tracing::error!("Telegram API error ({}): {}", status, err_text);
                    } else {
                        tracing::info!("✅ Telegram alert dispatched successfully.");
                    }
                }
                Err(e) => tracing::error!("Failed to send Telegram alert: {}", e),
            }
        }
    }

    /// V2: Handle incoming Telegram commands (Poll-based)
    pub async fn handle_telegram_commands(
        self: Arc<Self>,
        metrics: Arc<BotMetrics>,
        wallet_mgr: Arc<WalletManager>,
        payer_pubkey: Pubkey,
        start_time: Instant,
    ) {
        let mut last_update_id = 0;
        let mut interval = tokio::time::interval(Duration::from_secs(3)); // Poll every 3 seconds

        loop {
            interval.tick().await;

            if let Some(config) = &self.telegram_config {
                let url = format!(
                    "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=2",
                    config.bot_token, last_update_id + 1
                );

                match self.client.get(&url).send().await {
                    Ok(resp) => {
                        if let Ok(json) = resp.json::<Value>().await {
                            if let Some(result) = json.get("result").and_then(|r: &Value| r.as_array()) {
                                for update in result {
                                    if let Some(update_id) = update.get("update_id").and_then(|id: &Value| id.as_i64()) {
                                        last_update_id = update_id;
                                    }

                                    if let Some(message) = update.get("message") {
                                        let chat_id = message.get("chat")
                                            .and_then(|c: &Value| c.get("id"))
                                            .and_then(|id: &Value| id.as_i64())
                                            .map(|id: i64| id.to_string())
                                            .unwrap_or_default();
                                        
                                        // Only respond to our configured chat
                                        if chat_id != config.chat_id { continue; }

                                        if let Some(text) = message.get("text").and_then(|t: &Value| t.as_str()) {
                                            match text {
                                                "/status" => {
                                                    let report = self.create_enhanced_status_message(&metrics, &wallet_mgr, &payer_pubkey, start_time).await;
                                                    self.send_alert(AlertSeverity::Info, "Status Request", &report, vec![]).await;
                                                }
                                                "/pause" => {
                                                    metrics.is_paused.store(true, Ordering::Relaxed);
                                                    self.send_alert(AlertSeverity::Warning, "Remote Control", "⏸ Trading PAUSED via Telegram.", vec![]).await;
                                                }
                                                "/resume" => {
                                                    metrics.is_paused.store(false, Ordering::Relaxed);
                                                    self.send_alert(AlertSeverity::Success, "Remote Control", "▶️ Trading RESUMED via Telegram.", vec![]).await;
                                                }
                                                "/balance" => {
                                                    if let Ok(bal) = wallet_mgr.get_sol_balance(&payer_pubkey) {
                                                        let sol = bal as f64 / 1e9;
                                                        self.send_alert(AlertSeverity::Info, "Balance Request", &format!("Current Wallet Balance: {:.6} SOL", sol), vec![]).await;
                                                    }
                                                }
                                                "/help" => {
                                                    let help_text = "<b>Available Commands:</b>\n/status - Full performance report\n/pause - Stop all trading\n/resume - Start trading again\n/balance - Check SOL balance";
                                                    self.send_alert(AlertSeverity::Info, "Bot Menu", help_text, vec![]).await;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => tracing::debug!("Telegram polling error: {}", e),
                }
            }
        }
    }

    async fn create_enhanced_status_message(
        &self,
        metrics: &BotMetrics,
        wallet_mgr: &WalletManager,
        payer_pubkey: &Pubkey,
        start_time: Instant
    ) -> String {
        let _detected = metrics.opportunities_detected.load(Ordering::Relaxed);
        let jito_success = metrics.execution_jito_success.load(Ordering::Relaxed);
        let rpc_success = metrics.execution_rpc_fallback_success.load(Ordering::Relaxed);
        let total_executions = jito_success + rpc_success;
        let exec_attempts = metrics.execution_attempts_total.load(Ordering::Relaxed);
        let rejected_sanity = metrics.opportunities_rejected_profit_sanity.load(Ordering::Relaxed);
        let rejected_safety = metrics.opportunities_rejected_safety.load(Ordering::Relaxed);
        let rejected_rug = metrics.opportunities_rejected_rug.load(Ordering::Relaxed);
        let rejected_slippage = metrics.opportunities_rejected_slippage.load(Ordering::Relaxed);
        
        let profit = metrics.total_profit_lamports.load(Ordering::Relaxed);
        let loss = metrics.total_loss_lamports.load(Ordering::Relaxed);
        let net_pnl = (profit as i64 - loss as i64) as f64 / 1e9;
        let gas = metrics.total_gas_spent.load(Ordering::Relaxed) as f64 / 1e9;
        let current_sol = wallet_mgr.get_sol_balance(payer_pubkey).unwrap_or(0) as f64 / 1e9;
        
        let success_rate = if exec_attempts > 0 {
            (total_executions as f64 / exec_attempts as f64) * 100.0
        } else {
            0.0
        };

        let uptime = start_time.elapsed();
        let uptime_str = format!(
            "{}h {}m",
            uptime.as_secs() / 3600,
            (uptime.as_secs() % 3600) / 60
        );

        let status_emoji = if metrics.is_paused.load(Ordering::Relaxed) { "⏸ (PAUSED)" } else { "🟢 (ACTIVE)" };

        format!(
            "<b>Live Performance Report</b>\n\
             ⏱ <b>Uptime:</b> {} | <b>Mode:</b> {}\n\n\
             🛡️ <b>SAFETY ANALYTICS</b>\n\
             - Rejected (Rug Shield): {}\n\
             - Rejected (Slippage): {}\n\
             - Rejected (Sanity): {}\n\
             - Rejected (Safety): {}\n\n\
             🚀 <b>EXECUTION STATS</b>\n\
             - Success Rate: <b>{:.1}%</b> ({} attempts)\n\
             - Successes: {} ({} Jito, {} RPC)\n\n\
             💰 <b>ECONOMICS</b>\n\
             - Gas Spent: {:.6} SOL\n\
             - Wallet: {:.4} SOL\n\
             - 💵 <b>NET P&L:</b> <code>{:.6} SOL</code>",
            uptime_str, status_emoji, rejected_rug, rejected_slippage, rejected_sanity, rejected_safety,
            success_rate, exec_attempts, total_executions, jito_success, rpc_success,
            gas, current_sol, net_pnl
        )
    }

    pub async fn send_final_report(&self, metrics: Arc<BotMetrics>, start_time: Instant) {
        let detected = metrics.opportunities_detected.load(Ordering::Relaxed);
        let jito_success = metrics.execution_jito_success.load(Ordering::Relaxed);
        let rpc_success = metrics.execution_rpc_fallback_success.load(Ordering::Relaxed);
        let total_executions = jito_success + rpc_success;
        let exec_attempts = metrics.execution_attempts_total.load(Ordering::Relaxed);
        let rejected_sanity = metrics.opportunities_rejected_profit_sanity.load(Ordering::Relaxed);
        let rejected_safety = metrics.opportunities_rejected_safety.load(Ordering::Relaxed);
        
        let profit = metrics.total_profit_lamports.load(Ordering::Relaxed);
        let loss = metrics.total_loss_lamports.load(Ordering::Relaxed);
        let net_pnl = (profit as i64 - loss as i64) as f64 / 1e9;
        let gas = metrics.total_gas_spent.load(Ordering::Relaxed) as f64 / 1e9;

        let success_rate = if exec_attempts > 0 {
            (total_executions as f64 / exec_attempts as f64) * 100.0
        } else {
            0.0
        };

        let uptime = start_time.elapsed();
        let uptime_str = format!(
            "{}h {}m {}s",
            uptime.as_secs() / 3600,
            (uptime.as_secs() % 3600) / 60,
            uptime.as_secs() % 60
        );

        let message = format!(
            "<b>Final Session Performance</b>\n\
             ⏱ <b>Uptime:</b> {}\n\n\
             📈 <b>ARBITRAGE STATS</b>\n\
             - Detected: {}\n\
             - Rejected (Sanity): {}\n\
             - Rejected (Safety): {}\n\n\
             🚀 <b>EXECUTION STATS</b>\n\
             - Total Attempts: {}\n\
             - Successes: {} ({} Jito, {} RPC)\n\
             - 🎯 Success Rate: <b>{:.1}%</b>\n\n\
             💰 <b>FINAL BALANCE</b>\n\
             - Gas Spent: {:.6} SOL\n\
             - 💵 <b>Net P&L:</b> <code>{:.6} SOL</code>",
            uptime_str, detected, rejected_sanity, rejected_safety, 
            exec_attempts, total_executions, jito_success, rpc_success,
            success_rate, gas, net_pnl
        );

        self.send_alert(
            AlertSeverity::Info,
            "Engine Shutdown Summary",
            &message,
            vec![
                Field { name: "Net PnL".to_string(), value: format!("{:.6} SOL", net_pnl), inline: true },
                Field { name: "Uptime".to_string(), value: uptime_str, inline: true },
                Field { name: "Success %".to_string(), value: format!("{:.1}%", success_rate), inline: true },
            ]
        ).await;
    }
}

/// Background task to monitor bot health and send summary alerts
pub async fn monitor_health(
    alerts: Arc<AlertManager>, 
    metrics: Arc<BotMetrics>,
    wallet_mgr: Arc<WalletManager>,
    payer_pubkey: Pubkey,
    start_time: Instant,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes for granular monitoring
    let mut last_processed_count = 0;
    let mut tick_count: u32 = 0;
    
    tracing::info!("🩺 Health monitor started (interval: 5m, report: 1h)");
    loop {
        interval.tick().await;
        tick_count += 1;
        tracing::debug!("🩺 Health monitor tick: {}", tick_count);
        
        let detected = metrics.opportunities_detected.load(Ordering::Relaxed);
        let jito_success = metrics.execution_jito_success.load(Ordering::Relaxed);
        let rpc_success = metrics.execution_rpc_fallback_success.load(Ordering::Relaxed);
        let total_executions = jito_success + rpc_success;
        let exec_attempts = metrics.execution_attempts_total.load(Ordering::Relaxed);

        // 1. WebSocket Health Check
        if detected == last_processed_count && detected > 0 {
             // We've detected things before, but no new ones in 5 mins
             // This might be a silent WS failure or just a dead market
             alerts.send_warning("WebSocket Stalled: No new opportunities detected in the last 5 minutes.").await;
        }
        last_processed_count = detected;

        // 2. SOL Balance Check
        if let Ok(balance) = wallet_mgr.get_sol_balance(&payer_pubkey) {
            let sol = balance as f64 / 1e9;
            if sol < 0.1 { // 0.1 SOL threshold
                alerts.send_alert(
                    AlertSeverity::Critical, 
                    "LOW GAS BALANCE", 
                    &format!("Payer balance is dangerously low: {:.4} SOL. Refill immediately to prevent trade failures.", sol),
                    vec![Field { name: "Balance".to_string(), value: format!("{:.4} SOL", sol), inline: true }]
                ).await;
            }
        }

        // 3. Execution Success Rate Check
        if exec_attempts > 0 {
            let success_rate = (total_executions as f64 / exec_attempts as f64) * 100.0;
            if success_rate < 50.0 && exec_attempts > 5 {
                alerts.send_alert(
                    AlertSeverity::Warning,
                    "LOW SUCCESS RATE",
                    &format!("Execution success rate is currently {:.1}%. Check Jito rate limits or RPC congestion.", success_rate),
                    vec![
                        Field { name: "Attempts".to_string(), value: exec_attempts.to_string(), inline: true },
                        Field { name: "Successes".to_string(), value: total_executions.to_string(), inline: true }
                    ]
                ).await;
            }
        }

        // 4. Hourly Summary
        if tick_count == 1 || tick_count % 12 == 0 {
            let message = alerts.create_enhanced_status_message(&metrics, &wallet_mgr, &payer_pubkey, start_time).await;
            
            tracing::info!("📊 Sending enhanced status report to Discord/Telegram...");
            alerts.send_alert(
                AlertSeverity::Success,
                "Hourly Performance Summary",
                &message,
                vec![]
            ).await;
        }
    }
}
