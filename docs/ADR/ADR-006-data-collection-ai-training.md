# ADR-006: Data Collection for AI Training

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

AI model requires real market data to learn profitable vs unprofitable arbitrage patterns. Need systematic data collection without impacting bot performance.

## Decision

Implement **dual CSV logging system**:
1. **market_data.csv**: Raw pool updates (all changes)
2. **arbitrage_data.csv**: Detected opportunities with 5-hop features

## Data Schema

### Arbitrage Data CSV
```csv
timestamp,num_hops,profit_lamports,input_amount,total_fees_bps,max_price_impact_bps,min_liquidity,route
```

**Features Collected**:
- Route metrics (hops, fees, impact)
- Liquidity constraints
- Expected profitability
- Abbreviated route path

## Collection Strategy

**Mode**: `DRY_RUN=true`, `SIMULATION=false`
- Connect to live Solana mainnet
- Detect real opportunities
- Log without executing trades
- 24+ hour collection periods

## Implementation

```rust
pub async fn record_arbitrage(&self, opp: ArbitrageOpportunity) {
    // Async CSV writes, non-blocking
}
```

## Consequences

### Positive
- Real market conditions data
- Separate files for different purposes
- Asynchronous writes (no performance impact)
- Continuous learning capability

### Negative
- CSV file growth (managed by rotation)
- Storage requirements (~2MB per 10k opportunities)

## Data Quality Targets

- **Minimum samples**: 1000 opportunities
- **Duration**: 24+ hours
- **Diversity**: Multiple market conditions

## Privacy & Security

- No private keys in CSV
- Only public blockchain data
- Mint addresses abbreviated in route

## Related ADRs
- ADR-004: AI Model Integration with ONNX
- ADR-009: Simulation Mode for Testing
