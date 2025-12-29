use serde::{Serialize, Deserialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PoolWeight {
    pub pool_address: Pubkey,
    pub weight: f64,
    pub last_update_ts: u64,
    pub update_count: u32,
    pub dna_score: u64,
}

impl PoolWeight {
    pub fn new(pool_address: Pubkey) -> Self {
        Self {
            pool_address,
            weight: 10.0, // Base weight
            last_update_ts: 0,
            update_count: 0,
            dna_score: 0,
        }
    }
}

pub mod weight_constants {
    pub const BASE_WEIGHT: f64 = 10.0;
    pub const ACTIVITY_BONUS: f64 = 5.0;
    pub const DNA_BONUS_MULTIPLIER: f64 = 1.0;
    pub const DECAY_PER_SEC: f64 = 0.1;
    pub const MAX_WEIGHT: f64 = 1000.0;
    pub const MIN_WEIGHT_TO_SUBSCRBE: f64 = 5.0;
}
