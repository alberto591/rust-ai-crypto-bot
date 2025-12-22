# ADR-005: Jito Bundle Execution

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Solana MEV requires atomic transaction bundles to prevent frontrunning. Standard RPC submission is vulnerable to MEV attacks and failed arbitrage attempts.

## Decision

Use **Jito Block Engine** for bundle submission:
- Atomic transaction bundles
- Priority fee (tip) mechanism
- MEV protection via private mempool

## Architecture

```rust
pub struct JitoClient {
    client: Arc<Mutex<SearcherClient>>,
    keypair: Arc<Keypair>,
}
```

**Methods**:
- `build_bundle_instructions()` - Create swap + tip instructions
- `build_and_send_bundle()` - Atomic submission
- `get_tip_accounts()` - Fetch tip addresses

## Bundle Structure

1. **Swap Instructions**: Sequential arbitrage swaps
2. **Jito Tip**: Payment to validator for inclusion
3. **Atomic Execution**: All-or-nothing settlement

## Alternatives Considered

### Standard RPC sendTransaction
**Rejected**: No atomicity, frontrunning risk, failed arbs waste SOL

### Serum Crank
**Rejected**: Limited to Serum DEX, not cross-DEX

### Jito (CHOSEN)
**Pros**:
- Atomic bundles
- MEV protection
- Validator tip mechanism
- Production-ready infrastructure

**Cons**:
- Centralization risk (Jito dependency)
- Additional fees (tips)
- Network requirement

## Consequences

### Positive
- Protected from frontrunning
- Failed arbs don't waste gas
- Higher success rate
- Competitive advantage

### Negative
- Dependency on Jito availability
- Must pay tips for bundle inclusion
- Requires authentication

## Safety Mechanisms

- **DRY_RUN mode**: Test without real transactions
- **Simulation**: Pre-validate bundles before submission
- **Tip calculation**: Dynamic based on profitability

## Related ADRs
- ADR-009: Simulation Mode for Testing
- ADR-008: Port Abstractions (ExecutionPort)
