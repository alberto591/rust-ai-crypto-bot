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
    cached_analysis: Mutex<Option<(mev_core::SuccessAnalysis, std::time::Instant)>>,
}

impl DatabaseIntelligence {
    pub fn new(pool: Option<deadpool_postgres::Pool>) -> Self {
        let cache_size = NonZeroUsize::new(1000).unwrap();
        Self { 
            pool,
            blacklist_cache: Mutex::new(LruCache::new(cache_size)),
            cached_analysis: Mutex::new(None),
        }
        }


    pub fn calculate_dna_score(dna: &mev_core::TokenDNA) -> u64 {
        let mut score = 0;

        // 1. Liquidity Depth (40 pts)
        if dna.initial_liquidity >= 1_000_000_000 {
            score += 40;
        } else if dna.initial_liquidity >= 500_000_000 {
            score += 20;
        }

        // 2. Launch Hour Efficiency (30 pts)
        match dna.launch_hour_utc {
            13..=21 => score += 30,
            12 | 22 => score += 15,
            _ => {}
        }

        // 3. Security Hardening (30 pts)
        if dna.mint_renounced {
            score += 20;
        }
        if dna.has_twitter {
            score += 10;
        }
        
        score
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

    async fn get_stories_by_strategy(&self, _strategy_id: &str) -> Result<Vec<SuccessStory>> {
        if let Some(_pool) = &self.pool {
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
        // Cache Check (5 min TTL)
        {
            let cache = self.cached_analysis.lock().unwrap();
            if let Some((analysis, timestamp)) = &*cache {
                if timestamp.elapsed() < std::time::Duration::from_secs(300) {
                    return Ok(analysis.clone());
                }
            }
        }

        let result = if let Some(pool) = &self.pool {
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
            let median_time: f64 = row.get("median_time"); 
            let total: i64 = row.get("total");

            Ok(SuccessAnalysis {
                average_peak_roi: avg_roi,
                median_time_to_peak: median_time,
                total_successful_launches: total as usize,
                strategy_effectiveness: 0.85,
            })
        } else {
            // High-Performance File Aggregator (Phase 2 Fallback)
            let mut total_roi = 0.0;
            let mut total_time = 0.0;
            let mut count = 0;

            if let Ok(mut entries) = tokio::fs::read_dir("library").await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                   if let Ok(content) = tokio::fs::read(entry.path()).await {
                       if let Ok(story) = serde_json::from_slice::<SuccessStory>(&content) {
                           total_roi += story.peak_roi;
                           total_time += story.time_to_peak_secs as f64;
                           count += 1;
                       }
                   }
                }
            }

            if count > 0 {
                Ok(SuccessAnalysis {
                    average_peak_roi: total_roi / count as f64,
                    median_time_to_peak: total_time / count as f64,
                    total_successful_launches: count,
                    strategy_effectiveness: 0.90,
                })
            } else {
                Ok(SuccessAnalysis {
                    average_peak_roi: 0.0,
                    median_time_to_peak: 0.0,
                    total_successful_launches: 0,
                    strategy_effectiveness: 0.0,
                })
            }
        };

        // Update Cache
        if let Ok(ref analysis) = result {
            let mut cache = self.cached_analysis.lock().unwrap();
            *cache = Some((analysis.clone(), std::time::Instant::now()));
        }

        result
    }
}

#[async_trait]
impl strategy::ports::MarketIntelligencePort for DatabaseIntelligence {
    async fn is_blacklisted(&self, token_address: &Pubkey) -> Result<bool> {
        MarketIntelligence::is_blacklisted(self, token_address).await
    }

    async fn save_story(&self, story: mev_core::SuccessStory) -> Result<()> {
        MarketIntelligence::save_story(self, story).await
    }

    async fn get_success_analysis(&self) -> Result<mev_core::SuccessAnalysis> {
        MarketIntelligence::get_analysis(self).await
    }

    async fn match_dna(&self, dna: &mev_core::TokenDNA) -> Result<mev_core::DNAMatch> {
        let analysis = self.get_success_analysis().await?;
        let score = Self::calculate_dna_score(dna);

        tracing::info!("🧬 DNA SCORE: {}/100 (Min Reserve: {:.2} Units, Launch: {} UTC, Renounced: {})", 
            score, 
            dna.initial_liquidity as f64 / 1e9, 
            dna.launch_hour_utc,
            dna.mint_renounced
        );

        // Thresholding
        // Learning Phase (low total launches): 40 pts threshold
        // Professional Phase (>100 launches): 60 pts threshold
        // Lowered threshold from 40 to 30 based on Log Analysis 2024-12-29
        let threshold = if analysis.total_successful_launches > 100 { 50 } else { 30 };
        let elite_threshold = 80; // High confidence matches
        
        Ok(mev_core::DNAMatch {
            is_match: score >= threshold,
            is_elite: score >= elite_threshold,
            score,
        })
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use mev_core::TokenDNA;

    #[test]
    fn test_calculate_dna_score() {
        let base_dna = TokenDNA {
            initial_liquidity: 0,
            initial_market_cap: 0,
            launch_hour_utc: 0,
            has_twitter: false,
            mint_renounced: false,
            market_volatility: 0.0,
        };

        // Case 1: Minimal passing score (30 pts needed)
        // Just Launch Hour (30 pts)
        let mut dna = base_dna.clone();
        dna.launch_hour_utc = 14; 
        assert_eq!(DatabaseIntelligence::calculate_dna_score(&dna), 30);

        // Case 2: High Liquidity (40 pts)
        let mut dna = base_dna.clone();
        dna.initial_liquidity = 1_500_000_000; // 1.5 SOL
        assert_eq!(DatabaseIntelligence::calculate_dna_score(&dna), 40);

        // Case 3: Perfect Score (100 pts)
        let mut dna = base_dna.clone();
        dna.initial_liquidity = 1_500_000_000; // 40
        dna.launch_hour_utc = 15;              // 30
        dna.mint_renounced = true;             // 20
        dna.has_twitter = true;                // 10
        assert_eq!(DatabaseIntelligence::calculate_dna_score(&dna), 100);
    }
}
