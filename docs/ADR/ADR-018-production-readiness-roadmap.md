# ADR-018: Production Readiness Roadmap

**Status:** Accepted
**Date:** 2024-12-25
**Author:** Antigravity

## 1. Context (The "Why")
The Solana MEV bot has successfully completed Phase 1 (Dynamic Jito Tipping) and Phase 2 (Dynamic Slippage) and is currently undergoing a 10-hour LiveMicro production test on mainnet. To transition from a testing/experimental prototype to a robust, high-availability production system, we need a structured path for infrastructure hardening, performance optimization, and risk management.

Without this roadmap, the system remains vulnerable to:
- "Flying blind" during market volatility due to lack of real-time metrics.
- Slow incident response without an automated alerting system.
- Execution failures or financial risk from unvetted tokens and liquidity rugs.
- Lost opportunities due to sub-optimal gRPC and RPC latencies.

## 2. Decision
Adopt a 4-Phase implementation strategy for transition to full production:

### Phase 0: Immediate Fix (Strategy Validation)
- **Configuration**: Implement `PoolConfig` and verified triangular paths (SOL/JUP/USDC, SOL/RAY/USDC, etc.).
- **Metrics**: Implement local `BotMetrics` summary (Print on exit/interval).
- **Simulation**: 24-hour simulation run to prove opportunity detection.

### Phase 1: Foundation (Observability & Risk)
- **Telemetry**: Prometheus exporter (:9090) for structured performance tracking.
- **Alerting**: Discord/Telegram webhooks for critical incidents and success notifications.
- **Risk Management**: Implement `RiskManager` with daily limits, position sizing, and circuit breakers.

### Phase 2: Validation (Live Micro-Test)
- **LiveMicro**: Execute trades with small real capital (0.01 SOL) to verify slippage and win rates.
- **Analysis**: Statistical validation of strategy profitability before scaling.

### Phase 3: Optimization & Scaling
- **Latency**: Lock-free hot paths and zero-copy builders.
- **DEX Expansion**: Full Orca Whirlpool and Meteora support for cross-DEX arbitrage.
- **Infrastructure**: RPC failover and gRPC optimization.

## 3. Rationale (The "Proof")
- **Observability First**: We prioritize monitoring over performance because "you cannot optimize what you cannot measure."
- **Safety Over Speed**: Advanced safety checks (Phase 1) are prioritized alongside performance to protect principal capital during mainnet execution.
- **Iterative Deployment**: The 4-phase approach allows for incremental verification of each component during LiveMicro runs before moving to LiveProduction.

## 4. Consequences
- **Positive**: Increased stability, reduced financial risk from "bad" pools, and measurable performance improvements.
- **Negative/Trade-offs**: Increased codebase complexity; additional infrastructure costs (Prometheus, Grafana, VPS for dashboard).
- **Latency Impact**: Advanced safety checks may add <1ms of latency, which is acceptable compared to the risk of executing against a rug pool.

## 5. Wiring Check (No Dead Code)
- [x] Initial roadmap artifact created in `production_roadmap.md`
- [ ] Phase 3 Prometheus integration (next task)
- [ ] Phase 5 Token Validator updates (next task)
- [x] ADR-018 documentation completed
