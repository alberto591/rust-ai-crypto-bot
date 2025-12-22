# ADR-007: Price Impact Filtering

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Large trades can cause significant price slippage, making theoretically profitable arbitrage unprofitable in practice. Need to filter opportunities before execution.

## Decision

Implement **pre-execution price impact filtering**:
- Calculate impact for each swap step
- Reject paths with >1% impact on any single hop
- Track max impact across entire route

## Formula

```rust
price_impact = amount_in / (reserve_in + amount_in)
```

Where:
- `amount_in`: Trade size
- `reserve_in`: Pool reserves for input token

## Threshold

**Maximum allowed**: 1% (100 basis points) per hop

**Rationale**:
- Keeps effective price close to spot price
- Reduces slippage losses
- Prevents pool manipulation flagging

## Implementation

```rust
let price_impact = calculate_price_impact(current_amount, res_in);
if price_impact > 0.01 {  // 1%
    debug!("Skipping path due to high price impact: {:.2}%", price_impact * 100.0);
    continue;
}
```

## Consequences

### Positive
- Prevents unprofitable trades despite math saying profitable
- Protects against flash crash scenarios
- Improves execution quality
- Reduces failed transactions

### Negative
- May miss opportunities in low-liquidity pools
- Conservative (could tune higher for aggressive strategies)

## Alternative Thresholds Considered

- **0.5%**: Too conservative, misses valid trades
- **2%**: Too aggressive, frequent failures
- **1% (CHOSEN)**: Balanced approach

## Metrics Tracked

- `max_price_impact_bps`: Stored in `ArbitrageOpportunity`
- Used by AI model as input feature
- Logged for analysis

## Related ADRs
- ADR-002: Five-Hop Arbitrage Strategy
- ADR-004: AI Model Integration
