use prometheus::{Counter, Histogram, IntGauge, Registry, TextEncoder, Encoder, HistogramOpts};
use lazy_static::lazy_static;
use warp::Filter;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    // Opportunity metrics
    pub static ref OPPORTUNITIES_TOTAL: Counter = Counter::new(
        "opportunities_detected_total", 
        "Total arbitrage opportunities detected"
    ).unwrap();
    
    pub static ref OPPORTUNITIES_PROFITABLE: Counter = Counter::new(
        "opportunities_profitable_total",
        "Opportunities with positive expected profit"
    ).unwrap();
    
    pub static ref TRADES_EXECUTED: Counter = Counter::new(
        "trades_executed_total",
        "Total trades successfully executed"
    ).unwrap();
    
    // Profitability metrics
    pub static ref PROFIT_LAMPORTS: Counter = Counter::new(
        "profit_lamports_total",
        "Total profit in lamports"
    ).unwrap();
    
    pub static ref LOSS_LAMPORTS: Counter = Counter::new(
        "loss_lamports_total",
        "Total loss in lamports"
    ).unwrap();
    
    pub static ref GAS_SPENT_LAMPORTS: Counter = Counter::new(
        "gas_spent_lamports_total",
        "Total gas/fees spent in lamports"
    ).unwrap();
    
    // Slippage tracking
    pub static ref ACTUAL_SLIPPAGE_BPS: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "actual_slippage_bps",
            "Actual slippage experienced in basis points"
        )
    ).unwrap();
    
    // Latency metrics
    pub static ref DETECTION_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "detection_latency_ms",
            "Time from price update to opportunity detection"
        )
    ).unwrap();
    
    pub static ref EXECUTION_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "execution_latency_ms",
            "Time from detection to transaction sent"
        )
    ).unwrap();
    
    // Health metrics
    pub static ref WEBSOCKET_STATUS: IntGauge = IntGauge::new(
        "websocket_connected",
        "WebSocket connection status (1=connected, 0=disconnected)"
    ).unwrap();
    
    pub static ref RPC_ERRORS: Counter = Counter::new(
        "rpc_errors_total",
        "Total RPC errors encountered"
    ).unwrap();
    
    // Risk management metrics
    pub static ref CIRCUIT_BREAKER_TRIGGERS: Counter = Counter::new(
        "circuit_breaker_triggers_total",
        "Number of times circuit breaker was triggered"
    ).unwrap();
    
    pub static ref DAILY_PNL_LAMPORTS: IntGauge = IntGauge::new(
        "daily_pnl_lamports",
        "Current daily profit/loss in lamports"
    ).unwrap();
    
    pub static ref SAFETY_REJECTIONS: Counter = Counter::new(
        "safety_rejections_total",
        "Opportunities rejected by safety checks"
    ).unwrap();
}

pub fn init_metrics() {
    REGISTRY.register(Box::new(OPPORTUNITIES_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(OPPORTUNITIES_PROFITABLE.clone())).unwrap();
    REGISTRY.register(Box::new(TRADES_EXECUTED.clone())).unwrap();
    REGISTRY.register(Box::new(PROFIT_LAMPORTS.clone())).unwrap();
    REGISTRY.register(Box::new(LOSS_LAMPORTS.clone())).unwrap();
    REGISTRY.register(Box::new(GAS_SPENT_LAMPORTS.clone())).unwrap();
    REGISTRY.register(Box::new(ACTUAL_SLIPPAGE_BPS.clone())).unwrap();
    REGISTRY.register(Box::new(DETECTION_LATENCY.clone())).unwrap();
    REGISTRY.register(Box::new(EXECUTION_LATENCY.clone())).unwrap();
    REGISTRY.register(Box::new(WEBSOCKET_STATUS.clone())).unwrap();
    REGISTRY.register(Box::new(RPC_ERRORS.clone())).unwrap();
    REGISTRY.register(Box::new(CIRCUIT_BREAKER_TRIGGERS.clone())).unwrap();
    REGISTRY.register(Box::new(DAILY_PNL_LAMPORTS.clone())).unwrap();
    REGISTRY.register(Box::new(SAFETY_REJECTIONS.clone())).unwrap();
}

/// Start metrics HTTP server
pub async fn serve_metrics() {
    let metrics_route = warp::any()
        .map(|| {
            let encoder = TextEncoder::new();
            let mut buffer = Vec::new();
            encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });
    
    tracing::info!("📊 Prometheus metrics server starting on 0.0.0.0:8080");
    warp::serve(metrics_route)
        .run(([0, 0, 0, 0], 8080))
        .await;
}
