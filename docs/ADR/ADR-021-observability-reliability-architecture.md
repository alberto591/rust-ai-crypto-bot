# ADR-021: Observability & Reliability Architecture

## Context
As the Mojito Bot transitioned into 24/7 high-frequency trading (HFT) on Solana, the need for real-time observability and rigorous reliability verification became critical. Previous versions relied on terminal logs, which were insufficient for tracking micro-second latencies and complex "Success DNA" hit rates during market volatility.

## Decision
We have implemented a dual-layered observability stack and a formalized reliability testing suite.

### 1. 3-Panel TUI "Command Center"
To provide immediate visual feedback without overhead, the terminal interface has been expanded into three distinct functional zones:
- **Live Token Discovery**: real-time feed of Pump.fun and Raydium pool detections, color-coded by program ID.
- **Strategic Opportunities**: detailed list of detected arbitrage cycles, including token hops and expected profit.
- **HFT Vitals**: Header-level display of current detection latency (ms), simulated PnL, and monitored pool counts.

### 2. Expanded Zero-Copy Telemetry
To support deep analysis via Grafana/Prometheus without impacting hot-path execution, we implemented:
- **DNA Match Tracking**: dedicated counters for standard `dna_matches_total` and high-alpha `dna_elite_matches_total` (Golden Ratio/Hour).
- **Mojito Throughput**: total token discovery volume tracking via `discovery_tokens_detected_total`.
- **Zero-Copy Schema**: All telemetry updates move via `Arc<T>` wrappers, ensuring zero heap allocations during detection cycles.

### 3. HFT Reliability Suite
A multi-stage testing architecture ensures 100% compliance with HFT performance requirements:
- **Instruction Patching Proofs**: Unit tests verify that the `patch_raydium_swap` logic correctly offsets `amount_in` and `min_amount_out` without rebuilding the entire transaction.
- **DNA Gating Validation**: Formalized tests for the 0.3 Golden Ratio and 23:00 UTC temporal filters.
- **ROI Ceiling Verification**: Proof-of-implementation for the 500% "Elite" vs. 50% "Standard" profit ceilings.

## Consequences
- **Positive**: Total visibility into bot health and strategic hit rates. Zero-latency impact for telemetry data collection.
- **Positive**: 100% test coverage for the core HFT optimization logic (patching/DNA gating).
- **Negative**: Increased complexity in `engine/src/main.rs` and `engine/src/discovery.rs` due to TUI state synchronization.
- **Negative**: Requirement for larger Prometheus scrape intervals to avoid blocking the metrics server.

## Status
**ACCEPTED** - Implemented and verified in Production (Vultr NJ) as of 2025-12-28.
