use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use parking_lot::RwLock;
use std::collections::VecDeque;

const MAX_SAMPLES: usize = 20;

pub struct VolatilityTracker {
    // Map of pool address to a deque of price samples
    price_history: RwLock<HashMap<Pubkey, VecDeque<f64>>>,
}

impl Default for VolatilityTracker {
    fn default() -> Self {
        Self::new()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_tracker_new() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Should return 0.0 for unknown pool
        assert_eq!(tracker.get_volatility_factor(pool), 0.0);
    }

    #[test]
    fn test_add_sample() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Add samples
        tracker.add_sample(pool, 100.0);
        tracker.add_sample(pool, 105.0);
        tracker.add_sample(pool, 95.0);
        
        // Should return 0.0 with less than 5 samples
        assert_eq!(tracker.get_volatility_factor(pool), 0.0);
    }

    #[test]
    fn test_volatility_calculation_stable_price() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Add 10 stable price samples (all 100.0)
        for _ in 0..10 {
            tracker.add_sample(pool, 100.0);
        }
        
        // Volatility should be 0.0 for stable prices
        let volatility = tracker.get_volatility_factor(pool);
        assert!(volatility < 0.001, "Stable price volatility should be near zero, got {}", volatility);
    }

    #[test]
    fn test_volatility_calculation_volatile_price() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Add volatile price samples
        let prices = vec![100.0, 150.0, 80.0, 120.0, 90.0, 110.0, 140.0, 95.0];
        for price in prices {
            tracker.add_sample(pool, price);
        }
        
        // Volatility should be > 0 for volatile prices
        let volatility = tracker.get_volatility_factor(pool);
        assert!(volatility > 0.1, "Volatile prices should have significant volatility, got {}", volatility);
    }

    #[test]
    fn test_max_samples_window() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Add more than MAX_SAMPLES (20) samples
        for i in 0..25 {
            tracker.add_sample(pool, 100.0 + i as f64);
        }
        
        // Verify we can still get volatility (doesn't panic)
        let volatility = tracker.get_volatility_factor(pool);
        assert!(volatility >= 0.0);
        
        // Verify the oldest samples were evicted by checking the history size
        let history = tracker.price_history.read();
        let samples = history.get(&pool).unwrap();
        assert_eq!(samples.len(), MAX_SAMPLES, "Should maintain max {} samples", MAX_SAMPLES);
        
        // Verify oldest sample was evicted (first sample was 100.0, should be gone)
        // Newest samples should be retained (120.0-124.0)
        assert!(!samples.contains(&100.0), "Oldest sample should be evicted");
        assert!(samples.contains(&124.0), "Newest sample should be retained");
    }

    #[test]
    fn test_multiple_pools_independent() {
        let tracker = VolatilityTracker::new();
        let pool1 = Pubkey::new_unique();
        let pool2 = Pubkey::new_unique();
        
        // Add stable prices to pool1
        for _ in 0..10 {
            tracker.add_sample(pool1, 100.0);
        }
        
        // Add volatile prices to pool2
        let volatile_prices = vec![100.0, 150.0, 80.0, 120.0, 90.0, 110.0, 140.0, 95.0];
        for price in volatile_prices {
            tracker.add_sample(pool2, price);
        }
        
        // Pool1 should have low volatility
        let vol1 = tracker.get_volatility_factor(pool1);
        assert!(vol1 < 0.001, "Pool1 volatility should be near zero");
        
        // Pool2 should have high volatility
        let vol2 = tracker.get_volatility_factor(pool2);
        assert!(vol2 > 0.1, "Pool2 volatility should be significant");
    }

    #[test]
    fn test_normalized_volatility() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Add samples with known standard deviation
        // Mean = 100, std_dev = 10, normalized vol = 10/100 = 0.1
        let prices = vec![90.0, 95.0, 100.0, 105.0, 110.0];
        for price in prices {
            tracker.add_sample(pool, price);
        }
        
        let volatility = tracker.get_volatility_factor(pool);
        // Should be approximately 0.071 (actual std_dev is ~7.07)
        assert!(volatility > 0.05 && volatility < 0.10, 
            "Normalized volatility should be around 0.071, got {}", volatility);
    }

    #[test]
    fn test_zero_mean_edge_case() {
        let tracker = VolatilityTracker::new();
        let pool =Pubkey::new_unique();
        
        // Edge case: all prices are 0.0
        for _ in 0..10 {
            tracker.add_sample(pool, 0.0);
        }
        
        // Should return 0.0 for zero mean
        let volatility = tracker.get_volatility_factor(pool);
        assert_eq!(volatility, 0.0, "Zero mean should result in 0.0 volatility");
    }

    #[test]
    fn test_insufficient_samples_threshold() {
        let tracker = VolatilityTracker::new();
        let pool = Pubkey::new_unique();
        
        // Add exactly 4 samples (below the 5 sample threshold)
        for i in 1..=4 {
            tracker.add_sample(pool, i as f64 * 10.0);
        }
        
        // Should return 0.0 with insufficient samples
        assert_eq!(tracker.get_volatility_factor(pool), 0.0, 
            "Should return 0.0 with less than 5 samples");
        
        // Add one more sample to reach threshold
        tracker.add_sample(pool, 50.0);
        
        // Now should calculate volatility
        let volatility = tracker.get_volatility_factor(pool);
        assert!(volatility > 0.0, "Should calculate volatility with 5+ samples");
    }
}
