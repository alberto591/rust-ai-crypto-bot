use prometheus::{Counter, CounterVec, Histogram, IntGauge, Registry, TextEncoder, Encoder, HistogramOpts, Opts};
use lazy_static::lazy_static;

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

    // DNA & Discovery Metrics
    pub static ref DNA_MATCHES_TOTAL: Counter = Counter::new(
        "dna_matches_total",
        "Total number of standard successful DNA matches"
    ).unwrap();

    pub static ref DNA_ELITE_MATCHES_TOTAL: Counter = Counter::new(
        "dna_elite_matches_total",
        "Total number of Elite (Golden Ratio/Hour) DNA matches"
    ).unwrap();

    pub static ref DISCOVERY_TOKENS_TOTAL: Counter = Counter::new(
        "discovery_tokens_detected_total", 
        "Total number of new tokens detected by Mojito"
    ).unwrap();

    // Cache Performance Metrics
    pub static ref SAFETY_CACHE_HITS: Counter = Counter::new(
        "safety_cache_hits_total",
        "Total safety check cache hits"
    ).unwrap();

    pub static ref SAFETY_CACHE_MISSES: Counter = Counter::new(
        "safety_cache_misses_total",
        "Total safety check_cache misses"
    ).unwrap();

    pub static ref POOL_DEDUP_SKIPS: Counter = Counter::new(
        "pool_dedup_skips_total",
        "Total pools skipped due to deduplication"
    ).unwrap();

    // Strategy & Execution Reliability
    pub static ref JITO_BUNDLE_ERRORS: CounterVec = CounterVec::new(
        Opts::new("jito_bundle_errors_total", "Total Jito bundle submission errors"),
        &["endpoint_id"]
    ).unwrap();

    pub static ref SAFETY_FAILURES: CounterVec = CounterVec::new(
        Opts::new("safety_failures_total", "Total safety check failures with reason labels"),
        &["reason"]
    ).unwrap();

    pub static ref DISCOVERY_ERRORS: CounterVec = CounterVec::new(
        Opts::new("discovery_errors_total", "Total discovery/hydration errors"),
        &["type"]
    ).unwrap();

    pub static ref DISCOVERY_CACHE_HITS: Counter = Counter::new(
        "discovery_cache_hits_total",
        "Total signature cache hits in discovery"
    ).unwrap();
    
    pub static ref OPPORTUNITIES_NON_DNA_TOTAL: Counter = Counter::new(
        "opportunities_non_dna_total",
        "Total opportunities that did NOT match DNA success patterns"
    ).unwrap();

    pub static ref ROUTE_DEPTH_HISTOGRAM: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "route_depth_distribution",
            "Distribution of profitable arbitrage route depth (hop count)"
        ).buckets(vec![2.0, 3.0, 4.0, 5.0, 6.0])
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
    REGISTRY.register(Box::new(DNA_MATCHES_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(DNA_ELITE_MATCHES_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(DISCOVERY_TOKENS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(SAFETY_CACHE_HITS.clone())).unwrap();
    REGISTRY.register(Box::new(SAFETY_CACHE_MISSES.clone())).unwrap();
    REGISTRY.register(Box::new(POOL_DEDUP_SKIPS.clone())).unwrap();
    REGISTRY.register(Box::new(JITO_BUNDLE_ERRORS.clone())).unwrap();
    REGISTRY.register(Box::new(SAFETY_FAILURES.clone())).unwrap();
    REGISTRY.register(Box::new(DISCOVERY_ERRORS.clone())).unwrap();
    REGISTRY.register(Box::new(DISCOVERY_CACHE_HITS.clone())).unwrap();
    REGISTRY.register(Box::new(OPPORTUNITIES_NON_DNA_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(ROUTE_DEPTH_HISTOGRAM.clone())).unwrap();
}
