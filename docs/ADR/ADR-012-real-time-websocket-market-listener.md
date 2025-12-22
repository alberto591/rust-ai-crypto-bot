# ADR-012: Real-time WebSocket Market Listener

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

High-frequency arbitrage requires sub-second latency for market updates. Pull-based mechanisms (HTTP Polling) are too slow and consume excessive RPC credits. We need a push-based mechanism to receive pool reserve updates as they happen on-chain.

## Decision

Implement a dedicated **WebSocket (WSS) Listener** using `tokio-tungstenite` that subscribes to Solana `accountSubscribe` notifications for Raydium V4 pools.

- **Stream-based processing**: Use MPSC channels to pipe updates from the listener thread to the main arbitrage thread.
- **Binary Zero-copy Decoding**: Deserialize raw Base64 account data directly into the `AmmInfo` struct for maximum performance.
- **Connection Resilience**: Implement Ping/Pong heartbeats and automatic reconnection logic.

## Architecture

```rust
pub struct WebSocketListener {
    ws_url: String,
    tx: mpsc::Sender<MarketUpdate>,
    monitored_pools: Vec<Pubkey>,
}

// Subscription mapping to track pool IDs
let mut sub_map: HashMap<u64, Pubkey> = HashMap::new();
```

## Alternatives Considered

### HTTP Polling (getMultipleAccounts)
**Rejected**: Latency > 1s, high RPC credit usage, missing many fast market fluctuations.

### gRPC (Yellowstone Geyser)
**Rejected for current phase**: High infrastructure cost and complexity; WebSocket is sufficient for initial production launch on public/private standard nodes.

### WebSocket Listener (CHOSEN)
**Pros**:
- Sub-100ms update latency.
- Low bandwidth and RPC credit footprint.
- Real-time reactivity to market moves.

**Cons**:
- Sensitive to network instability and rate limits.
- Requires robust state management for subscription IDs.

## Consequences

### Positive
- Enables the bot to compete in the "latency game".
- Reliable event-driven arbitrage triggers.
- Lower operational costs (RPC credits).

### Negative
- Public RPCs often disconnect WebSocket streams under load; requires private RPC for stability.

## Related ADRs
- ADR-003: Graph-based Market Representation (Consumes these updates)
- ADR-009: Simulation Mode (Mocks this listener)
