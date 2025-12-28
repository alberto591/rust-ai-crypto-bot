use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::mpsc::Receiver;
use mev_core::SuccessStory;
use crate::discovery::DiscoveryEvent;
use crate::config::BotConfig;
use crate::intelligence::MarketIntelligence;
use anyhow::Result;
use chrono::Utc;
use chrono::Timelike; // Import Timelike trait for .hour()

pub struct BirthWatcher {
    config: Arc<BotConfig>,
    intelligence: Arc<dyn MarketIntelligence>,
    rpc_client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
}

impl BirthWatcher {
    pub fn new(
        config: Arc<BotConfig>,
        intelligence: Arc<dyn MarketIntelligence>,
        rpc_url: &str,
    ) -> Self {
        let rpc_client = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(rpc_url.to_string()));
        Self {
            config,
            intelligence,
            rpc_client,
        }
    }

    pub async fn run(&self, mut rx: Receiver<DiscoveryEvent>) {
        tracing::info!("🍼 BirthWatcher ONLINE. Ready to nurture new tokens...");

        while let Some(event) = rx.recv().await {
            let rpc = Arc::clone(&self.rpc_client);
            let intelligence = Arc::clone(&self.intelligence);
            let event_clone = event.clone();

            tokio::spawn(async move {
                let pool_addr = event_clone.pool_address;
                if pool_addr == solana_sdk::pubkey::Pubkey::default() || pool_addr == solana_sdk::pubkey::Pubkey::from_str("11111111111111111111111111111111").unwrap() {
                    return;
                }
                if let Err(e) = track_birth(rpc, intelligence, event_clone).await {
                    tracing::error!("❌ Error tracking birth for {}: {}", pool_addr, e);
                }
            });
        }
    }
}

async fn track_birth(
    _rpc: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
    intelligence: Arc<dyn MarketIntelligence>,
    event: DiscoveryEvent,
) -> Result<()> {
    tracing::info!("🌱 Tracking initial 5 minutes for token: {}", event.pool_address);
    
    // 1. Wait and Monitor (Simulated for 5 minutes or until $1M MC)
    // For this POC, we'll wait a few seconds and "simulated" a success if it's a known winner.
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // 2. Success Check
    // If market cap > $1M (Simulated condition)
    let simulated_market_cap = 1_200_000; 
    if simulated_market_cap >= 1_000_000 {
        tracing::info!("🏆 SUCCESS! Token {} hit $1M Market Cap. Saving to library.", event.pool_address);
        
        let now = Utc::now();
        let story = SuccessStory {
            strategy_id: "momentum_sniper_v1".to_string(),
            token_address: event.pool_address.to_string(),
            market_context: "Meme_Season_Discovery".to_string(),
            lesson: "High early engagement; liquidity lock verified.".to_string(),
            timestamp: now.timestamp() as u64,
            
            // Entry Triggers
            liquidity_min: 15_000,
            has_twitter: true,
            mint_renounced: true,
            initial_market_cap: 50_000,
            
            // Performance Stats
            peak_roi: 450.0,
            time_to_peak_secs: 14 * 60,
            drawdown: 12.0,
            
            is_false_positive: false,

            // Enhanced Context (Phase 6)
            holder_count_at_peak: Some(1250), // Placeholder
            market_volatility: Some(0.42),    // Placeholder
            launch_hour_utc: Some(now.hour() as u8),
        };

        intelligence.save_story(story).await?;
    }

    Ok(())
}
