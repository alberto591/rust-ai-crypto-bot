use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use crate::metrics::BotMetrics;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::time::{Instant, Duration};
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
            
            if let Err(e) = self.client.post(&url).json(&payload).send().await {
                tracing::error!("Failed to send Telegram alert: {}", e);
            }
        }
    }
}

/// Background task to monitor bot health and send summary alerts
pub async fn monitor_health(
    alerts: Arc<AlertManager>, 
    metrics: Arc<BotMetrics>,
    wallet_mgr: Arc<WalletManager>,
    payer_pubkey: Pubkey,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes for granular monitoring
    let mut last_processed_count = 0;
    let mut tick_count: u32 = 0;
    
    loop {
        interval.tick().await;
        tick_count += 1;
        
        let detected = metrics.opportunities_detected.load(Ordering::Relaxed);
        let jito_success = metrics.execution_jito_success.load(Ordering::Relaxed);
        let rpc_success = metrics.execution_rpc_fallback_success.load(Ordering::Relaxed);
        let total_executions = jito_success + rpc_success;
        let exec_attempts = metrics.execution_attempts_total.load(Ordering::Relaxed);
        
        let profit = metrics.total_profit_lamports.load(Ordering::Relaxed);
        let loss = metrics.total_loss_lamports.load(Ordering::Relaxed);
        let net_pnl = (profit as i64 - loss as i64) as f64 / 1e9;

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

        // 4. Hourly Summary (using a modulo or simple counter)
        // Since we run every 5 mins, index 12 is roughly 1 hour
        if tick_count % 12 == 0 {
            let message = format!(
                "Hourly Status Report:\n- Detected: {}\n- Executed: {} ({} Jito, {} RPC)\n- Net P&L: {:.4} SOL",
                detected, total_executions, jito_success, rpc_success, net_pnl
            );
            alerts.send_alert(
                AlertSeverity::Success,
                "Hourly Performance Summary",
                &message,
                vec![
                    Field { name: "PnL".to_string(), value: format!("{:.4} SOL", net_pnl), inline: true },
                    Field { name: "Executions".to_string(), value: total_executions.to_string(), inline: true }
                ]
            ).await;
        }
    }
}
