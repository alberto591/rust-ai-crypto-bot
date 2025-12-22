# ADR-009: Simulation Mode for Testing

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Need to develop and test arbitrage strategies without risking real capital or paying transaction fees.

## Decision

Implement **two-tier testing system**:

### 1. DRY_RUN Mode
- Connects to real or simulated blockchain
- Detects real opportunities
- **Does NOT submit transactions**
- Logs what would have been executed

### 2. SIMULATION Mode
- Uses **mock pool data** instead of live RPC
- Faster execution (no network calls)
- Deterministic testing scenarios

## Environment Variables

```bash
# Local testing with mock data
SIMULATION=true
DRY_RUN=true

# Live data collection without trading
SIMULATION=false
DRY_RUN=true

# Production (DANGEROUS)
SIMULATION=false
DRY_RUN=false
```

## Implementation

**DRY_RUN Check**:
```rust
if std::env::var("DRY_RUN").is_ok() {
    info!("DRY RUN | Bundle built successfully");
    return Ok("dry_run_id".to_string());
}
```

**SIMULATION Data Source**:
```rust
let listener = if cfg.simulation_mode {
    // Use mock pools
    SimulatedListener::new()
} else {
    // Connect to Solana RPC
    SolanaListener::new(rpc_url).await?
};
```

## Testing Scenarios

### Mock Pool Setup
```rust
create_pool_update(sol, usdc, 1_000 * SOL, 50_000 * USDC, 30); // 1:50 ratio
create_pool_update(usdc, ray, 50_000 * USDC, 25_000 * RAY, 30); // 2:1 ratio
// Creates profitable triangle when combined
```

## Consequences

### Positive
- Safe development environment
- Fast iteration cycles
- Reproducible test cases
- No blockchain costs during development

### Negative
- Mock data may not reflect real market conditions
- Need to maintain test scenarios

## Safety Guardrails

1. **All modes protect private keys** (never logged)
2. **DRY_RUN default** in development
3. **Explicit opt-in** for production mode
4. **Visual indicators** in TUI showing mode

## Data Collection Use Case

**Configuration for 24h collection**:
```bash
SIMULATION=false  # Real market data
DRY_RUN=true     # Don't execute trades
```

Provides real arbitrage opportunities without financial risk.

## Related ADRs
- ADR-006: Data Collection for AI Training
- ADR-010: TUI Dashboard Design
