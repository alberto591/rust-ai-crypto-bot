# ADR-002: Five-Hop Arbitrage Strategy

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Multi-DEX arbitrage on Solana requires detecting profitable cycles across multiple trading pairs. Traditional triangular arbitrage (3 hops) may miss opportunities in longer paths.

## Decision

Implement **recursive DFS-based cycle detection** supporting up to **5 hops**:
- Triangular: SOL → USDC → RAY → SOL (3 hops)
- Quadrilateral: SOL → USDC → BONK → RAY → SOL (4 hops)
- 5-hop: Maximum depth for balance between opportunity discovery and computation

## Rationale

1. **Market Fragmentation**: Solana has multiple DEXs (Raydium, Orca, Serum)
2. **Deeper Opportunities**: Longer paths may have less competition
3. **Computational Feasibility**: 5 hops is tractable with proper optimization

## Algorithm

```rust
fn find_cycles_recursive(
    current_node, start_node, current_amount,
    visited, current_steps, best_opp, remaining_hops
)
```

- DFS with backtracking
- Tracks cumulative fees, price impact, and liquidity
- Returns best opportunity across all paths

## Consequences

### Positive
- Discovers more arbitrage opportunities than 3-hop
- Configurable depth via `max_hops` parameter
- Handles various market structures

### Negative
- Higher computational cost (exponential in worst case)
- Need to filter by price impact to avoid unprofitable long paths

## Optimization Strategies
- Early termination on high price impact (>1%)
- Minimum profit threshold
- Graph pruning for inactive pools

## Related ADRs
- ADR-003: Graph-Based Market Representation
- ADR-007: Price Impact Filtering
