use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use parking_lot::RwLock;
use std::collections::VecDeque;

const MAX_SAMPLES: usize = 20;

pub struct VolatilityTracker {
    // Map of pool address to a deque of price samples
    price_history: RwLock<HashMap<Pubkey, VecDeque<f64>>>,
}

impl VolatilityTracker {
    pub fn new() -> Self {
        Self {
            price_history: RwLock::new(HashMap::new()),
        }
    }

    /// Adds a price sample for a pool
    pub fn add_sample(&self, pool: Pubkey, price: f64) {
        let mut history = self.price_history.write();
        let samples = history.entry(pool).or_insert_with(|| VecDeque::with_capacity(MAX_SAMPLES));
        
        if samples.len() >= MAX_SAMPLES {
            samples.pop_front();
        }
        samples.push_back(price);
    }

    /// Calculates volatility factor (normalized standard deviation)
    pub fn get_volatility_factor(&self, pool: Pubkey) -> f64 {
        let history = self.price_history.read();
        let samples = match history.get(&pool) {
            Some(s) if s.len() >= 5 => s, // Need at least 5 samples for meaningful volatility
            _ => return 0.0,
        };

        let n = samples.len() as f64;
        let mean = samples.iter().sum::<f64>() / n;
        
        let variance = samples.iter()
            .map(|&p| {
                let diff = p - mean;
                diff * diff
            })
            .sum::<f64>() / n;
        
        let std_dev = variance.sqrt();
        
        // Return normalized volatility (std_dev / mean)
        if mean > 0.0 {
            std_dev / mean
        } else {
            0.0
        }
    }
}
