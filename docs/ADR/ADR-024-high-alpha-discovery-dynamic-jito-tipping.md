# ADR-024: High-Alpha Discovery & Dynamic Jito Tipping

**Status:** Accepted
**Date:** 2025-12-28
**Author:** Antigravity

## 1. Context (The "Why")
Solana congestion frequently causes Jito bundles to fail if tips are static and below the current network "floor". Additionally, the most profitable trades (high-alpha) occur during Pump.fun token migrations from bonding curves to Raydium. Capturing these requires precise discovery and competitive tipping.

## 2. Decision
Implemented advanced discovery and dynamic execution tuning:
* **Discovery**: Added `PumpFunMigrationWatcher` in `discovery.rs` to detect "InitializePool" events originating from Pump.fun migrations.
* **Execution**: Integrated Jito's HTTP API (`/api/v1/bundles/tip_floor`) to fetch real-time tipping floors (25th-99th percentiles).
* **Tipping**: Replaced static tiered tipping with a dynamic model: `max(planned_tip, tip_floor_50th_ema * 1.05)`.
* **Budget**: Added automatic `ComputeBudget` instruction tuning to minimize CU wastage and rejection risk.

## 3. Rationale (The "Proof")
* Dynamic tipping ensures we win bundles during spiky congestion without permanently overpaying.
* Pump.fun migration sniping is the highest ROI activity for MEV bots on Solana in Q4 2025.
* Compute Budget tuning is a "Best Practice" for fast network processing.

## 4. Consequences
* **Positive:** Massive increase in bundle "land rate"; focus on high-alpha migration events.
* **Negative/Trade-offs:** Dependency on Jito's HTTP API for floors (falls back to static defaults if API is down).

## 5. Wiring Check (No Dead Code)
- [x] Discovery logic in `engine/src/discovery.rs`
- [x] Jito API client in `executor/src/jito.rs`
- [x] Compute budget instructions in `executor/src/jito.rs`
- [x] Verified via `test_jito_tip_floor_query`
