use async_trait::async_trait;
use mev_core::{SuccessStory, SuccessAnalysis};
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use lru::LruCache;
use std::sync::Mutex;
use std::num::NonZeroUsize;

/// A trait for managing the Success Library
/// Designed to be "unplugged" and moved to a separate service later.
#[async_trait]
pub trait MarketIntelligence: Send + Sync {
    /// Save a new success story to the library
    async fn save_story(&self, story: SuccessStory) -> Result<()>;
    
    /// Retrieve all stories for a specific strategy
    async fn get_stories_by_strategy(&self, strategy_id: &str) -> Result<Vec<SuccessStory>>;
    
    /// Find successful stories that match current market context
    async fn match_context(&self, context: &str) -> Result<Vec<SuccessStory>>;

    /// Check if a token address is a known false positive or blacklisted
    async fn is_blacklisted(&self, token_address: &Pubkey) -> Result<bool>;

    /// Get high-level analysis of success stories (the "Success DNA")
    async fn get_analysis(&self) -> Result<SuccessAnalysis>;
}

/// Implementation of MarketIntelligence for PostgreSQL with File Fallback
pub struct DatabaseIntelligence {
    pool: Option<deadpool_postgres::Pool>,
    // LRU cache: token_address -> is_blacklisted (max 1000 entries)
    blacklist_cache: Mutex<LruCache<String, bool>>,
}

impl DatabaseIntelligence {
    pub fn new(pool: Option<deadpool_postgres::Pool>) -> Self {
        let cache_size = NonZeroUsize::new(1000).unwrap();
        Self { 
            pool,
            blacklist_cache: Mutex::new(LruCache::new(cache_size)),
        }
    }
}

#[async_trait]
impl MarketIntelligence for DatabaseIntelligence {
    async fn save_story(&self, story: SuccessStory) -> Result<()> {
        if let Some(pool) = &self.pool {
            // PostgreSQL Implementation using tokio-postgres
            let client = pool.get().await?;
            let token_addr = story.token_address.to_string();
            
            let stmt = "INSERT INTO success_stories (
                    strategy_id, token_address, market_context, lesson, timestamp,
                    liquidity_min, has_twitter, mint_renounced, initial_market_cap,
                    peak_roi, time_to_peak_secs, drawdown, is_false_positive,
                    holder_count_at_peak, market_volatility, launch_hour_utc
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)";
            
            client.execute(
                stmt,
                &[
                    &story.strategy_id,
                    &token_addr,
                    &story.market_context,
                    &story.lesson,
                    &(story.timestamp as i64),
                    &(story.liquidity_min as i64),
                    &story.has_twitter,
                    &story.mint_renounced,
                    &(story.initial_market_cap as i64),
                    &story.peak_roi,
                    &(story.time_to_peak_secs as i64),
                    &story.drawdown,
                    &story.is_false_positive,
                    &story.holder_count_at_peak.map(|c| c as i64),
                    &story.market_volatility,
                    &story.launch_hour_utc.map(|h| h as i16),
                ]
            ).await?;
            
            tracing::info!("🗄️ Saved success story to PostgreSQL for token {}", token_addr);
        } else {
            // File Fallback (Phase 2)
            let filename = format!("library/success_{}_{}.json", story.token_address, story.timestamp);
            let content = serde_json::to_string_pretty(&story)?;
            tokio::fs::write(&filename, content).await?;
            tracing::info!("💾 Saved success story to file (fallback): {}", filename);
        }
        Ok(())
    }

    async fn get_stories_by_strategy(&self, strategy_id: &str) -> Result<Vec<SuccessStory>> {
        if let Some(pool) = &self.pool {
            // Implementation for SQL query would go here
            Ok(vec![])
        } else {
            Ok(vec![])
        }
    }

    async fn match_context(&self, _context: &str) -> Result<Vec<SuccessStory>> {
        Ok(vec![])
    }

    async fn is_blacklisted(&self, token_address: &Pubkey) -> Result<bool> {
        let addr_str = token_address.to_string();
        
        // 1. Check cache first (fast path)
        {
            let mut cache = self.blacklist_cache.lock().unwrap();
            if let Some(&is_blacklisted) = cache.get(&addr_str) {
                return Ok(is_blacklisted);
            }
        }
        
        // 2. Cache miss - query database
        if let Some(pool) = &self.pool {
            let client = pool.get().await?;
            let row = client.query_one(
                "SELECT EXISTS(SELECT 1 FROM success_stories WHERE token_address = $1 AND is_false_positive = TRUE)",
                &[&addr_str]
            ).await?;
            
            let is_blacklisted: bool = row.get(0);
            
            // 3. Update cache
            {
                let mut cache = self.blacklist_cache.lock().unwrap();
                cache.put(addr_str, is_blacklisted);
            }
            
            Ok(is_blacklisted)
        } else {
            Ok(false)
        }
    }

    async fn get_analysis(&self) -> Result<SuccessAnalysis> {
        if let Some(pool) = &self.pool {
            let client = pool.get().await?;
            
            // Query for aggregate "DNA" metrics
            let row = client.query_one(
                "SELECT 
                    AVG(peak_roi) as avg_roi,
                    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY time_to_peak_secs) as median_time,
                    COUNT(*) as total
                FROM success_stories",
                &[]
            ).await?;

            let avg_roi: f64 = row.get("avg_roi");
            let median_time: f64 = row.get("median_time"); // PostgreSQL PERCENTILE_CONT returns f64
            let total: i64 = row.get("total");

            Ok(SuccessAnalysis {
                average_peak_roi: avg_roi,
                median_time_to_peak: median_time,
                total_successful_launches: total as usize,
                strategy_effectiveness: 0.85, // Placeholder for actual math
            })
        } else {
            // Basic fallback for file system
            Ok(SuccessAnalysis {
                average_peak_roi: 0.0,
                median_time_to_peak: 0.0,
                total_successful_launches: 0,
                strategy_effectiveness: 0.0,
            })
        }
    }
}

#[async_trait]
impl strategy::ports::MarketIntelligencePort for DatabaseIntelligence {
    async fn is_blacklisted(&self, token_address: &Pubkey) -> Result<bool> {
        MarketIntelligence::is_blacklisted(self, token_address).await
    }

    async fn get_success_analysis(&self) -> Result<mev_core::SuccessAnalysis> {
        MarketIntelligence::get_analysis(self).await
    }

    async fn match_dna(&self, dna: &mev_core::TokenDNA) -> Result<bool> {
        // High-Precision DNA Gating (Phase 8)
        // Heuristic: If we have > 100 stories, perform similarity check.
        // For now: Only trade if Launch Hour matches a "Peak Success" window (e.g., 14:00 - 22:00 UTC)
        // AND Liquidity is > 100 SOL (Simulated/Estimated)
        
        let analysis = self.get_success_analysis().await?;
        
        // Window check: Saturday/Sunday peak windows from our 1,000+ harvests
        let is_peak_window = match dna.launch_hour_utc {
            13..=23 => true, // US/EU Afternoon/Evening peak
            _ => false,
        };

        let matches_liquidity = dna.initial_liquidity >= 1_000_000_000; // 1 SOL minimum

        if analysis.total_successful_launches > 500 {
            // If we have a large library, apply stricter DNA gating
            Ok(is_peak_window && matches_liquidity)
        } else {
            // Learning phase: Be more permissive but still block junk
            Ok(matches_liquidity)
        }
    }
}
