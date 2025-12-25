use tracing::{info, warn};

/// Capital scaling strategy based on performance
pub struct CapitalScaler {
    current_tier: CapitalTier,
    win_rate_threshold: f64,
    min_trades_for_promotion: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CapitalTier {
    Tier1, // 0.01 SOL - Initial production
    Tier2, // 0.05 SOL - After 70%+ win rate over 100 trades
    Tier3, // 0.1 SOL  - After 70%+ win rate over 200 trades
    Tier4, // 0.5 SOL  - After 75%+ win rate over 500 trades
}

impl CapitalTier {
    pub fn max_position_lamports(&self) -> u64 {
        match self {
            CapitalTier::Tier1 => 10_000_000,   // 0.01 SOL
            CapitalTier::Tier2 => 50_000_000,   // 0.05 SOL
            CapitalTier::Tier3 => 100_000_000,  // 0.1 SOL
            CapitalTier::Tier4 => 500_000_000,  // 0.5 SOL
        }
    }

    pub fn daily_profit_target_lamports(&self) -> u64 {
        match self {
            CapitalTier::Tier1 => 5_000_000,    // 0.005 SOL
            CapitalTier::Tier2 => 25_000_000,   // 0.025 SOL
            CapitalTier::Tier3 => 50_000_000,   // 0.05 SOL
            CapitalTier::Tier4 => 250_000_000,  // 0.25 SOL
        }
    }
}

impl CapitalScaler {
    pub fn new() -> Self {
        Self {
            current_tier: CapitalTier::Tier1,
            win_rate_threshold: 0.70,
            min_trades_for_promotion: 100,
        }
    }

    /// Evaluate if we should scale up capital based on performance
    pub fn should_scale_up(
        &self,
        total_trades: u64,
        winning_trades: u64,
    ) -> Option<CapitalTier> {
        if total_trades < self.min_trades_for_promotion {
            return None;
        }

        let win_rate = winning_trades as f64 / total_trades as f64;
        
        match self.current_tier {
            CapitalTier::Tier1 if win_rate >= 0.70 && total_trades >= 100 => {
                info!("✅ Promoting to Tier 2 (0.05 SOL) - Win rate: {:.1}%", win_rate * 100.0);
                Some(CapitalTier::Tier2)
            }
            CapitalTier::Tier2 if win_rate >= 0.70 && total_trades >= 200 => {
                info!("✅ Promoting to Tier 3 (0.1 SOL) - Win rate: {:.1}%", win_rate * 100.0);
                Some(CapitalTier::Tier3)
            }
            CapitalTier::Tier3 if win_rate >= 0.75 && total_trades >= 500 => {
                info!("✅ Promoting to Tier 4 (0.5 SOL) - Win rate: {:.1}%", win_rate * 100.0);
                Some(CapitalTier::Tier4)
            }
            _ => None,
        }
    }

    /// Downgrade tier if performance degrades
    pub fn should_scale_down(&self, win_rate: f64) -> Option<CapitalTier> {
        if win_rate < 0.50 {
            warn!("⚠️ Win rate below 50% - scaling down capital");
            match self.current_tier {
                CapitalTier::Tier4 => Some(CapitalTier::Tier3),
                CapitalTier::Tier3 => Some(CapitalTier::Tier2),
                CapitalTier::Tier2 => Some(CapitalTier::Tier1),
                CapitalTier::Tier1 => None,
            }
        } else {
            None
        }
    }

    pub fn get_max_position(&self) -> u64 {
        self.current_tier.max_position_lamports()
    }

    pub fn update_tier(&mut self, new_tier: CapitalTier) {
        self.current_tier = new_tier;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capital_scaling() {
        let scaler = CapitalScaler::new();
        
        // Should promote from Tier 1 to Tier 2 with 70%+ win rate over 100 trades
        let promotion = scaler.should_scale_up(100, 75);
        assert_eq!(promotion, Some(CapitalTier::Tier2));
        
        // Should not promote with insufficient trades
        let no_promotion = scaler.should_scale_up(50, 40);
        assert_eq!(no_promotion, None);
    }

    #[test]
    fn test_scale_down() {
        let mut scaler = CapitalScaler::new();
        scaler.update_tier(CapitalTier::Tier3);
        
        // Should downgrade with poor performance
        let downgrade = scaler.should_scale_down(0.45);
        assert_eq!(downgrade, Some(CapitalTier::Tier2));
    }
}
