use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};

pub struct RiskManager {
    // Daily limits
    pub max_daily_trades: u32,
    pub max_daily_volume_lamports: u64,
    pub max_daily_loss_lamports: u64,
    
    // Position limits
    pub max_position_size_lamports: u64,
    pub max_slippage_bps: u16,
    
    // Current state
    pub daily_trades: AtomicU32,
    pub daily_volume: AtomicU64,
    pub daily_loss: AtomicU64,
    
    // Circuit breaker
    pub consecutive_losses: AtomicU32,
    pub circuit_breaker_triggered: std::sync::atomic::AtomicBool,
}

impl RiskManager {
    pub fn new() -> Self {
        Self {
            max_daily_trades: 100,
            max_daily_volume_lamports: 2_000_000_000, // 2 SOL
            max_daily_loss_lamports: 50_000_000, // 0.05 SOL
            max_position_size_lamports: 20_000_000, // 0.02 SOL
            max_slippage_bps: 50, // 0.5%
            
            daily_trades: AtomicU32::new(0),
            daily_volume: AtomicU64::new(0),
            daily_loss: AtomicU64::new(0),
            consecutive_losses: AtomicU32::new(0),
            circuit_breaker_triggered: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    pub fn can_trade(&self, amount: u64) -> Result<(), RiskError> {
        // Check circuit breaker
        if self.circuit_breaker_triggered.load(Ordering::Relaxed) {
            return Err(RiskError::CircuitBreakerTripped);
        }
        
        // Check daily trade limit
        if self.daily_trades.load(Ordering::Relaxed) >= self.max_daily_trades {
            return Err(RiskError::DailyTradeLimitReached);
        }
        
        // Check daily volume limit
        let current_volume = self.daily_volume.load(Ordering::Relaxed);
        if current_volume + amount > self.max_daily_volume_lamports {
            return Err(RiskError::DailyVolumeLimitReached);
        }
        
        // Check position size
        if amount > self.max_position_size_lamports {
            return Err(RiskError::PositionSizeTooLarge);
        }
        
        // Check daily loss limit
        if self.daily_loss.load(Ordering::Relaxed) >= self.max_daily_loss_lamports {
            return Err(RiskError::DailyLossLimitReached);
        }
        
        Ok(())
    }
    
    pub fn record_trade(&self, amount: u64, profit: i64) {
        self.daily_trades.fetch_add(1, Ordering::Relaxed);
        self.daily_volume.fetch_add(amount, Ordering::Relaxed);
        
        if profit < 0 {
            self.daily_loss.fetch_add(profit.abs() as u64, Ordering::Relaxed);
            let losses = self.consecutive_losses.fetch_add(1, Ordering::Relaxed) + 1;
            
            // Trip circuit breaker after 5 consecutive losses
            if losses >= 5 {
                self.circuit_breaker_triggered.store(true, Ordering::Relaxed);
                tracing::error!("🚨 CIRCUIT BREAKER TRIGGERED after {} consecutive losses", losses);
            }
        } else {
            self.consecutive_losses.store(0, Ordering::Relaxed);
        }
    }
    
    pub fn reset_daily_limits(&self) {
        self.daily_trades.store(0, Ordering::Relaxed);
        self.daily_volume.store(0, Ordering::Relaxed);
        self.daily_loss.store(0, Ordering::Relaxed);
        self.consecutive_losses.store(0, Ordering::Relaxed);
        self.circuit_breaker_triggered.store(false, Ordering::Relaxed);
        tracing::info!("✅ Daily risk limits reset");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RiskError {
    #[error("Circuit breaker tripped")]
    CircuitBreakerTripped,
    #[error("Daily trade limit reached")]
    DailyTradeLimitReached,
    #[error("Daily volume limit reached")]
    DailyVolumeLimitReached,
    #[error("Daily loss limit reached")]
    DailyLossLimitReached,
    #[error("Position size too large")]
    PositionSizeTooLarge,
}
