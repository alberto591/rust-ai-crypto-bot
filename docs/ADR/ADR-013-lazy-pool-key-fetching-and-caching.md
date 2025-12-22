# ADR-013: Lazy Pool Key Fetching & Caching

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Raydium V4 swaps require 18+ specific account keys (Vaults, Authorities, OpenOrders, etc.). Fetching all these keys for every potential pool at startup is slow and memory-intensive, especially as the number of monitored pools scales.

## Decision

Implement a **Lazy Fetching and Caching strategy** for pool keys.

- **On-demand Fetching**: Pool keys are only fetched when an arbitrage opportunity is first detected in a pool.
- **Persistent Caching**: Once fetched, keys are stored in a `HashMap` in the main loop for immediate subsequent use.
- **Zero-copy Deserialization**: Use `bytemuck` to cast raw account data into `AmmInfo` to extract vault and order keys instantly.

## Architecture

```rust
pub struct PoolKeyFetcher {
    rpc: RpcClient,
}

// In main loop cache
let mut pool_key_cache: HashMap<Pubkey, RaydiumSwapKeys> = HashMap::new();

// Lazy Fetch Logic
if let Vacant(entry) = pool_key_cache.entry(pool_id) {
    let keys = fetcher.fetch_keys(pool_id).await?;
    entry.insert(keys);
}
```

## Alternatives Considered

### Full Pre-fetching at Static Initialization
**Rejected**: Fails to scale to hundreds of pools; makes startup very slow (minutes).

### Inclusion of Keys in WebSocket Updates
**Rejected**: Solana WebSockets only push state changes, not the full account layout; keys must be fetched separately.

### Lazy Fetching (CHOSEN)
**Pros**:
- Near-instant startup time.
- Optimized RPC usage (only fetch what is traded).
- Minimal memory footprint.

**Cons**:
- First trade on a new pool will have a ~100ms latency penalty due to the fetch.

## Consequences

### Positive
- The bot can scale to monitor thousands of pools dynamically.
- Clean separation between market state (Reserves) and market metadata (Pool Keys).

### Negative
- "Cold-start" penalty on the very first arbitrage cycle for any newly discovered pool.

## Related ADRs
- ADR-005: Jito Bundle Execution (Requires these keys for instruction building)
- ADR-011: Wallet Management (Uses these keys for ATA lookups)
