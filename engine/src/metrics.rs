use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use tracing::info;

/// Enhanced bot metrics with execution tracking
pub struct BotMetrics {
    // Opportunity tracking
    pub opportunities_detected: AtomicU64,
    pub opportunities_profitable: AtomicU64,
    pub opportunities_rejected_profit_sanity: AtomicU64,
    pub opportunities_rejected_safety: AtomicU64,
    pub opportunities_rejected_rug: AtomicU64,      // NEW: V2
    pub opportunities_rejected_slippage: AtomicU64, // NEW: V2
    
    // Execution tracking - NEW SECTION
    pub execution_attempts_total: AtomicU64,
    pub execution_jito_success: AtomicU64,
    pub execution_jito_failed: AtomicU64,
    pub execution_rpc_fallback_success: AtomicU64,
    pub execution_rpc_fallback_failed: AtomicU64,
    
    // Retry tracking - NEW SECTION
    pub retry_attempt_1_success: AtomicU64,  // First retry succeeded
    pub retry_attempt_2_success: AtomicU64,  // Second retry succeeded
    pub retry_attempt_3_success: AtomicU64,  // Third retry succeeded
    
    // Endpoint health - NEW SECTION
    pub endpoint_0_attempts: AtomicU64,
    pub endpoint_0_successes: AtomicU64,
    pub endpoint_1_attempts: AtomicU64,
    pub endpoint_1_successes: AtomicU64,
    pub endpoint_2_attempts: AtomicU64,
    pub endpoint_2_successes: AtomicU64,
    
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
    
    // Remote Control State - NEW: V2
    pub is_paused: std::sync::atomic::AtomicBool, 
}

impl strategy::ports::TelemetryPort for BotMetrics {
    fn log_opportunity(&self, profitable: bool) {
        self.log_opportunity(profitable);
    }
    fn log_profit_sanity_rejection(&self) {
        self.log_profit_sanity_rejection();
    }
    fn log_safety_rejection(&self) {
        self.log_safety_rejection();
    }
    fn log_rug_rejection(&self) {
        self.log_rug_rejection();
    }
    fn log_slippage_rejection(&self) {
        self.log_slippage_rejection();
    }
    fn log_execution_attempt(&self) {
        self.log_execution_attempt();
    }
    fn log_jito_success(&self) {
        self.log_jito_success();
    }
    fn log_jito_failed(&self) {
        self.log_jito_failed();
    }
    fn log_rpc_fallback_success(&self) {
        self.log_rpc_fallback_success();
    }
    fn log_rpc_fallback_failed(&self) {
        self.log_rpc_fallback_failed();
    }
    fn log_retry_success(&self, retry_number: usize) {
        self.log_retry_success(retry_number);
    }
    fn log_endpoint_attempt(&self, endpoint_index: usize) {
        self.log_endpoint_attempt(endpoint_index);
    }
    fn log_endpoint_success(&self, endpoint_index: usize) {
        self.log_endpoint_success(endpoint_index);
    }
    fn log_realized_pnl(&self, lamports: i64) {
        if lamports > 0 {
            self.total_profit_lamports.fetch_add(lamports as u64, Ordering::SeqCst);
        } else if lamports < 0 {
            self.total_loss_lamports.fetch_add(lamports.abs() as u64, Ordering::SeqCst);
        }
    }

    fn get_total_loss(&self) -> u64 {
        self.total_loss_lamports.load(Ordering::SeqCst)
    }

    fn get_win_rate(&self) -> f32 {
        let attempts = self.execution_attempts_total.load(Ordering::Relaxed) as f32;
        let success = (self.execution_jito_success.load(Ordering::Relaxed) + self.execution_rpc_fallback_success.load(Ordering::Relaxed)) as f32;
        if attempts > 0.0 {
            success / attempts
        } else {
            1.0 // Assume 100% win rate if no trades made yet to avoid aggressive scaling down
        }
    }
}

impl BotMetrics {
    pub fn new() -> Self {
        Self {
            // Opportunity tracking
            opportunities_detected: AtomicU64::new(0),
            opportunities_profitable: AtomicU64::new(0),
            opportunities_rejected_profit_sanity: AtomicU64::new(0),
            opportunities_rejected_safety: AtomicU64::new(0),
            opportunities_rejected_rug: AtomicU64::new(0),      // NEW: V2
            opportunities_rejected_slippage: AtomicU64::new(0), // NEW: V2
            
            // Execution tracking
            execution_attempts_total: AtomicU64::new(0),
            execution_jito_success: AtomicU64::new(0),
            execution_jito_failed: AtomicU64::new(0),
            execution_rpc_fallback_success: AtomicU64::new(0),
            execution_rpc_fallback_failed: AtomicU64::new(0),
            
            // Retry tracking
            retry_attempt_1_success: AtomicU64::new(0),
            retry_attempt_2_success: AtomicU64::new(0),
            retry_attempt_3_success: AtomicU64::new(0),
            
            // Endpoint health
            endpoint_0_attempts: AtomicU64::new(0),
            endpoint_0_successes: AtomicU64::new(0),
            endpoint_1_attempts: AtomicU64::new(0),
            endpoint_1_successes: AtomicU64::new(0),
            endpoint_2_attempts: AtomicU64::new(0),
            endpoint_2_successes: AtomicU64::new(0),
            
            // Performance tracking
            total_profit_lamports: AtomicU64::new(0),
            total_loss_lamports: AtomicU64::new(0),
            total_gas_spent: AtomicU64::new(0),
            
            // Latency tracking
            avg_detection_latency_ms: AtomicU32::new(0),
            avg_execution_latency_ms: AtomicU32::new(0),
            
            // Health tracking
            websocket_reconnects: AtomicU32::new(0),
            rpc_errors: AtomicU32::new(0),
            
            // Remote Control
            is_paused: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn log_opportunity(&self, profitable: bool) {
        self.opportunities_detected.fetch_add(1, Ordering::Relaxed);
        if profitable {
            self.opportunities_profitable.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn log_profit_sanity_rejection(&self) {
        self.opportunities_rejected_profit_sanity.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_safety_rejection(&self) {
        self.opportunities_rejected_safety.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_rug_rejection(&self) {
        self.opportunities_rejected_rug.fetch_add(1, Ordering::Relaxed);
    }

    pub fn log_slippage_rejection(&self) {
        self.opportunities_rejected_slippage.fetch_add(1, Ordering::Relaxed);
    }
    
    // NEW: Execution tracking methods
    pub fn log_execution_attempt(&self) {
        self.execution_attempts_total.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_jito_success(&self) {
        self.execution_jito_success.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_jito_failed(&self) {
        self.execution_jito_failed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_rpc_fallback_success(&self) {
        self.execution_rpc_fallback_success.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_rpc_fallback_failed(&self) {
        self.execution_rpc_fallback_failed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn log_retry_success(&self, retry_number: usize) {
        match retry_number {
            0 => { self.retry_attempt_1_success.fetch_add(1, Ordering::Relaxed); },
            1 => { self.retry_attempt_2_success.fetch_add(1, Ordering::Relaxed); },
            2 => { self.retry_attempt_3_success.fetch_add(1, Ordering::Relaxed); },
            _ => {}
        }
    }
    
    pub fn log_endpoint_attempt(&self, endpoint_index: usize) {
        match endpoint_index {
            0 => { self.endpoint_0_attempts.fetch_add(1, Ordering::Relaxed); },
            1 => { self.endpoint_1_attempts.fetch_add(1, Ordering::Relaxed); },
            2 => { self.endpoint_2_attempts.fetch_add(1, Ordering::Relaxed); },
            _ => {}
        }
    }
    
    pub fn log_endpoint_success(&self, endpoint_index: usize) {
        match endpoint_index {
            0 => { self.endpoint_0_successes.fetch_add(1, Ordering::Relaxed); },
            1 => { self.endpoint_1_successes.fetch_add(1, Ordering::Relaxed); },
            2 => { self.endpoint_2_successes.fetch_add(1, Ordering::Relaxed); },
            _ => {}
        }
    }
    
    pub fn print_summary(&self) {
        let detected = self.opportunities_detected.load(Ordering::Relaxed);
        let profitable = self.opportunities_profitable.load(Ordering::Relaxed);
        let rejected_sanity = self.opportunities_rejected_profit_sanity.load(Ordering::Relaxed);
        let rejected_safety = self.opportunities_rejected_safety.load(Ordering::Relaxed);
        
        let exec_total = self.execution_attempts_total.load(Ordering::Relaxed);
        let jito_ok = self.execution_jito_success.load(Ordering::Relaxed);
        let jito_fail = self.execution_jito_failed.load(Ordering::Relaxed);
        let rpc_ok = self.execution_rpc_fallback_success.load(Ordering::Relaxed);
        let rpc_fail = self.execution_rpc_fallback_failed.load(Ordering::Relaxed);
        
        println!("
╔════════════════════════════════════════════════════╗
║          BOT PERFORMANCE SUMMARY                   ║
╠════════════════════════════════════════════════════╣
║ OPPORTUNITIES                                      ║
║   Detected:           {:>14}                   ║
║   Profitable:         {:>14}                   ║
║   Rejected (Sanity):  {:>14}                   ║
║   Rejected (Safety):  {:>14}                   ║
╠════════════════════════════════════════════════════╣
║ EXECUTION                                          ║
║   Total Attempts:     {:>14}                   ║
║   Jito Success:       {:>14} ({:>5.1}%)          ║
║   Jito Failed:        {:>14} ({:>5.1}%)          ║
║   RPC Fallback OK:    {:>14} ({:>5.1}%)          ║
║   RPC Fallback Fail:  {:>14} ({:>5.1}%)          ║
╠════════════════════════════════════════════════════╣
║ PROFIT/LOSS                                        ║
║   Total Profit: {:>24.4} SOL              ║
║   Total Loss:   {:>24.4} SOL              ║
║   Net P&L:      {:>24.4} SOL              ║
╚════════════════════════════════════════════════════╝
        ",
            detected,
            profitable,
            rejected_sanity,
            rejected_safety,
            exec_total,
            jito_ok, if exec_total > 0 { (jito_ok as f64 / exec_total as f64) * 100.0 } else { 0.0 },
            jito_fail, if exec_total > 0 { (jito_fail as f64 / exec_total as f64) * 100.0 } else { 0.0 },
            rpc_ok, if exec_total > 0 { (rpc_ok as f64 / exec_total as f64) * 100.0 } else { 0.0 },
            rpc_fail, if exec_total > 0 { (rpc_fail as f64 / exec_total as f64) * 100.0 } else { 0.0 },
            self.total_profit_lamports.load(Ordering::Relaxed) as f64 / 1e9,
            self.total_loss_lamports.load(Ordering::Relaxed) as f64 / 1e9,
            (self.total_profit_lamports.load(Ordering::Relaxed) as i64 
             - self.total_loss_lamports.load(Ordering::Relaxed) as i64) as f64 / 1e9,
        );
    }

    pub fn print_periodic_update(&self) {
        let detected = self.opportunities_detected.load(Ordering::Relaxed);
        let profitable = self.opportunities_profitable.load(Ordering::Relaxed);
        let exec_total = self.execution_attempts_total.load(Ordering::Relaxed);
        let jito_ok = self.execution_jito_success.load(Ordering::Relaxed);
        let rpc_ok = self.execution_rpc_fallback_success.load(Ordering::Relaxed);
        let net = (self.total_profit_lamports.load(Ordering::Relaxed) as i64 
                  - self.total_loss_lamports.load(Ordering::Relaxed) as i64) as f64 / 1e9;

        info!("📈 [PERIODIC] Opps: {}/{} | Exec: {} ({} Jito ✅, {} RPC ✅) | PnL: {:.4} SOL",
            profitable, detected, exec_total, jito_ok, rpc_ok, net
        );
    }
    
    /// NEW: Print detailed execution stats
    pub fn print_execution_details(&self) {
        let retry_1 = self.retry_attempt_1_success.load(Ordering::Relaxed);
        let retry_2 = self.retry_attempt_2_success.load(Ordering::Relaxed);
        let retry_3 = self.retry_attempt_3_success.load(Ordering::Relaxed);
        
        let ep0_attempts = self.endpoint_0_attempts.load(Ordering::Relaxed);
        let ep0_success = self.endpoint_0_successes.load(Ordering::Relaxed);
        let ep1_attempts = self.endpoint_1_attempts.load(Ordering::Relaxed);
        let ep1_success = self.endpoint_1_successes.load(Ordering::Relaxed);
        let ep2_attempts = self.endpoint_2_attempts.load(Ordering::Relaxed);
        let ep2_success = self.endpoint_2_successes.load(Ordering::Relaxed);
        
        println!("
╔════════════════════════════════════════════════════╗
║          EXECUTION DETAILS                         ║
╠════════════════════════════════════════════════════╣
║ RETRY PERFORMANCE                                  ║
║   1st Retry Success:  {:>14}                   ║
║   2nd Retry Success:  {:>14}                   ║
║   3rd Retry Success:  {:>14}                   ║
╠════════════════════════════════════════════════════╣
║ ENDPOINT HEALTH                                    ║
║   Endpoint 0 (Amsterdam):                          ║
║     Attempts: {:>14}   Success: {:>8}        ║
║     Success Rate: {:>29.1}%                ║
║   Endpoint 1 (Frankfurt):                          ║
║     Attempts: {:>14}   Success: {:>8}        ║
║     Success Rate: {:>29.1}%                ║
║   Endpoint 2 (New York):                           ║
║     Attempts: {:>14}   Success: {:>8}        ║
║     Success Rate: {:>29.1}%                ║
╚════════════════════════════════════════════════════╝
        ",
            retry_1,
            retry_2,
            retry_3,
            ep0_attempts, ep0_success,
            if ep0_attempts > 0 { (ep0_success as f64 / ep0_attempts as f64) * 100.0 } else { 0.0 },
            ep1_attempts, ep1_success,
            if ep1_attempts > 0 { (ep1_success as f64 / ep1_attempts as f64) * 100.0 } else { 0.0 },
            ep2_attempts, ep2_success,
            if ep2_attempts > 0 { (ep2_success as f64 / ep2_attempts as f64) * 100.0 } else { 0.0 },
        );
    }
}
