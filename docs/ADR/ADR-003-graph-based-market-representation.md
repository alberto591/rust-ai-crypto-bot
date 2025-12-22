# ADR-003: Graph-Based Market Representation

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Need an efficient data structure to represent the Solana DEX ecosystem and perform pathfinding for arbitrage detection.

## Decision

Use **petgraph::DiGraph** (Directed Graph) where:
- **Vertices**: Token mint addresses (`Pubkey`)
- **Edges**: Liquidity pools (`PoolUpdate`) with bidirectional connections
- **Weights**: Pool reserves, fees, and metadata

## Alternatives Considered

### HashMap-based Custom Graph
**Pros**: No external dependency, full control  
**Cons**: Need to implement graph algorithms, more code to maintain

### Lightweight Graph Library
**Pros**: Simpler API  
**Cons**: Less mature, fewer algorithms

### petgraph (CHOSEN)
**Pros**: 
- Industry standard, battle-tested
- Rich algorithm library (DFS, BFS, shortest path)
- Actively maintained
- Zero-cost abstractions

**Cons**: 
- External dependency
- Slightly higher learning curve

## Implementation

```rust
pub struct ArbitrageStrategy {
    graph: Mutex<DiGraph<Pubkey, PoolUpdate>>,
    nodes: Mutex<HashMap<Pubkey, NodeIndex>>,
}
```

## Consequences

### Positive
- Efficient pathfinding algorithms
- Thread-safe with Mutex
- Dynamic graph updates
- Well-documented API

### Negative
- Dependency on external crate
- Lock contention on high-frequency updates (mitigated by fine-grained locking)

## Performance Characteristics
- Graph update: O(1)
- DFS traversal: O(V + E)
- Memory: Minimal (only active pools tracked)

## Related ADRs
- ADR-002: Five-Hop Arbitrage Strategy
