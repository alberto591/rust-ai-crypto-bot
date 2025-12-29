use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use mev_core::pool_weight::{PoolWeight, weight_constants::*};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PoolScoringEngine {
    weights: DashMap<Pubkey, PoolWeight>,
    pool: Option<deadpool_postgres::Pool>,
}

use std::str::FromStr;

impl PoolScoringEngine {
    pub fn new(pool: Option<deadpool_postgres::Pool>) -> Self {
        Self {
            weights: DashMap::new(),
            pool,
        }
    }

    pub async fn init_db(&self) -> anyhow::Result<()> {
        if let Some(pool) = &self.pool {
            let client = pool.get().await?;
            client.batch_execute("
                CREATE TABLE IF NOT EXISTS pool_weights (
                    pool_address TEXT PRIMARY KEY,
                    weight DOUBLE PRECISION NOT NULL DEFAULT 10.0,
                    last_update_ts BIGINT NOT NULL,
                    update_count INTEGER NOT NULL DEFAULT 0,
                    dna_score INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_pool_weights_value ON pool_weights (weight DESC);
            ").await?;
            tracing::info!("🗄️ Pool weights table verified/created.");
        }
        Ok(())
    }

    pub async fn load_from_db(&self) -> anyhow::Result<()> {
        if let Some(pool) = &self.pool {
            let client = pool.get().await?;
            let rows = client.query("SELECT * FROM pool_weights WHERE weight > 15.0 ORDER BY weight DESC LIMIT 500", &[]).await?;
            
            for row in rows {
                let addr_str: String = row.get("pool_address");
                let pool_addr = Pubkey::from_str(&addr_str).unwrap_or_default();
                let weight = PoolWeight {
                    pool_address: pool_addr,
                    weight: row.get("weight"),
                    last_update_ts: row.get::<_, i64>("last_update_ts") as u64,
                    update_count: row.get::<_, i32>("update_count") as u32,
                    dna_score: row.get::<_, i32>("dna_score") as u64,
                };
                self.weights.insert(pool_addr, weight);
            }
            tracing::info!("📥 Loaded {} weights from PostgreSQL.", self.weights.len());
        }
        Ok(())
    }

    pub async fn sync_to_db(&self) -> anyhow::Result<()> {
        if let Some(pool) = &self.pool {
            let client = pool.get().await?;
            let snapshot: Vec<PoolWeight> = self.weights.iter().map(|kv| kv.value().clone()).collect();
            
            for w in snapshot {
                if w.weight < 11.0 && w.update_count < 5 { continue; } // Don't persist trash
                
                client.execute(
                    "INSERT INTO pool_weights (pool_address, weight, last_update_ts, update_count, dna_score)
                     VALUES ($1, $2, $3, $4, $5)
                     ON CONFLICT (pool_address) DO UPDATE SET
                     weight = $2, last_update_ts = $3, update_count = $4, dna_score = $5",
                    &[
                        &w.pool_address.to_string(),
                        &w.weight,
                        &(w.last_update_ts as i64),
                        &(w.update_count as i32),
                        &(w.dna_score as i32),
                    ]
                ).await?;
            }
            tracing::info!("📤 Synced weights to PostgreSQL.");
        }
        Ok(())
    }

    pub fn update_activity(&self, pool_address: Pubkey) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        let mut entry = self.weights.entry(pool_address).or_insert_with(|| PoolWeight::new(pool_address));
        
        // 1. Activity Bonus
        entry.weight = (entry.weight + ACTIVITY_BONUS).min(MAX_WEIGHT);
        entry.update_count += 1;
        entry.last_update_ts = now;
    }

    pub fn update_dna_score(&self, pool_address: Pubkey, dna_score: u64) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut entry = self.weights.entry(pool_address).or_insert_with(|| PoolWeight::new(pool_address));
        entry.dna_score = dna_score;
        entry.last_update_ts = now;
        
        // Adjust weight based on DNA score
        let dna_bonus = (dna_score as f64) * DNA_BONUS_MULTIPLIER;
        entry.weight = (entry.weight + dna_bonus).min(MAX_WEIGHT);
    }

    pub fn decay_weights(&self) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        self.weights.retain(|_pk, weight| {
            let elapsed = now.saturating_sub(weight.last_update_ts);
            if elapsed > 0 {
                let actual_decay = (elapsed as f64) * DECAY_PER_SEC;
                weight.weight = (weight.weight - actual_decay).max(0.0);
            }
            
            // Retain if weight is above 1.0 or last update was within 1 hour
            // This prevents the map from growing indefinitely
            weight.weight > 1.0 || elapsed < 3600 
        });
    }

    pub fn get_weight(&self, pool_address: &Pubkey) -> f64 {
        self.weights.get(pool_address).map(|w| w.weight).unwrap_or(BASE_WEIGHT)
    }

    pub fn get_top_pools(&self, limit: usize) -> Vec<PoolWeight> {
        let mut all_weights: Vec<PoolWeight> = self.weights.iter().map(|kv| kv.value().clone()).collect();
        all_weights.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        all_weights.into_iter().take(limit).collect()
    }
}
