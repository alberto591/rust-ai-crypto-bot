use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use tracing::info;

pub struct BotMetrics {
    // Opportunity tracking
    pub opportunities_detected: AtomicU64,
    pub opportunities_profitable: AtomicU64,
    pub opportunities_executed: AtomicU64,
    
    // Performance tracking
    pub total_profit_lamports: AtomicU64,
    pub total_loss_lamports: AtomicU64,
    pub total_gas_spent: AtomicU64,
    
    // Latency tracking
    pub avg_detection_latency_ms: AtomicU32,
    pub avg_execution_latency_ms: AtomicU32,
    
    // Health tracking
    pub websocket_reconnects: AtomicU32,
    pub rpc_errors: AtomicU32,
}

impl BotMetrics {
    pub fn new() -> Self {
        Self {
            opportunities_detected: AtomicU64::new(0),
            opportunities_profitable: AtomicU64::new(0),
            opportunities_executed: AtomicU64::new(0),
            total_profit_lamports: AtomicU64::new(0),
            total_loss_lamports: AtomicU64::new(0),
            total_gas_spent: AtomicU64::new(0),
            avg_detection_latency_ms: AtomicU32::new(0),
            avg_execution_latency_ms: AtomicU32::new(0),
            websocket_reconnects: AtomicU32::new(0),
            rpc_errors: AtomicU32::new(0),
        }
    }

    pub fn log_opportunity(&self, profitable: bool, executed: bool) {
        self.opportunities_detected.fetch_add(1, Ordering::Relaxed);
        if profitable {
            self.opportunities_profitable.fetch_add(1, Ordering::Relaxed);
        }
        if executed {
            self.opportunities_executed.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn print_summary(&self) {
        let detected = self.opportunities_detected.load(Ordering::Relaxed);
        let profitable = self.opportunities_profitable.load(Ordering::Relaxed);
        let executed = self.opportunities_executed.load(Ordering::Relaxed);
        
        println!("
╔════════════════════════════════════════╗
║       BOT PERFORMANCE SUMMARY          ║
╠════════════════════════════════════════╣
║ Opportunities Detected: {:>14} ║
║ Profitable (after gas): {:>14} ║
║ Successfully Executed:  {:>14} ║
║ Win Rate: {:>27.1}% ║
╠════════════════════════════════════════╣
║ Total Profit: {:>20.4} SOL ║
║ Total Loss:   {:>20.4} SOL ║
║ Net P&L:      {:>20.4} SOL ║
╚════════════════════════════════════════╝
        ",
            detected,
            profitable,
            executed,
            if executed > 0 { (profitable as f64 / executed as f64) * 100.0 } else { 0.0 },
            self.total_profit_lamports.load(Ordering::Relaxed) as f64 / 1e9,
            self.total_loss_lamports.load(Ordering::Relaxed) as f64 / 1e9,
            (self.total_profit_lamports.load(Ordering::Relaxed) as i64 
             - self.total_loss_lamports.load(Ordering::Relaxed) as i64) as f64 / 1e9,
        );
    }

    pub fn print_periodic_update(&self) {
        let detected = self.opportunities_detected.load(Ordering::Relaxed);
        let profitable = self.opportunities_profitable.load(Ordering::Relaxed);
        let executed = self.opportunities_executed.load(Ordering::Relaxed);
        let profit = self.total_profit_lamports.load(Ordering::Relaxed) as f64 / 1e9;
        let loss = self.total_loss_lamports.load(Ordering::Relaxed) as f64 / 1e9;
        let net = (self.total_profit_lamports.load(Ordering::Relaxed) as i64 
                  - self.total_loss_lamports.load(Ordering::Relaxed) as i64) as f64 / 1e9;

        info!("📈 [PERIODIC REPORT] Stats: Detected: {}, Profitable: {}, Executed: {} | PnL: {:.4} SOL (Net: {:.4})",
            detected, profitable, executed, profit - loss, net
        );
    }
}
