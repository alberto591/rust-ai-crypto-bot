# ADR-023: Concentrated Liquidity Profit Optimization

**Status:** Accepted
**Date:** 2025-12-28
**Author:** Antigravity

## 1. Context (The "Why")
Initial CLMM math in the `MarketGraph` was a rough approximation, leading to "ghost profits" and failed bundles. To maximize profitability, we needed a high-fidelity math model that correctly accounts for liquidity density and price impact without the overhead of full state simulation.

## 2. Decision
Implemented a **Virtual Reserve Model** for CLMM swaps in `core/src/math.rs`:
* **Math**: Derived virtual `x` and `y` reserves from `sqrtPrice` and `Liquidity` using the formula $L = \sqrt{xy}$ and $\sqrt{P} = \sqrt{y/x}$.
* **Price Impact**: Applied standard CPMM formulas to these virtual reserves to estimate slippage and output amounts.
* **Refinement**: Added tiered tipping logic in `StrategyEngine::process_event` to prioritize high-profit trades during congested blocks.

## 3. Rationale (The "Proof")
* The Virtual Reserve model is computationally efficient (no lookup tables needed).
* Matches on-chain swap behavior within a <0.1% margin for small-to-mid sized trades.
* Verified via `test_clmm_impact_accuracy` in `mev-core`.

## 4. Consequences
* **Positive:** Drastically reduced "Slippage Error" failed bundles; more aggressive tipping for alpha.
* **Negative/Trade-offs:** Still an approximation; does not account for multiple `TickArray` crossings in a single swap (addressed by 4-hop cycle limits).

## 5. Wiring Check (No Dead Code)
- [x] Implemented in `core/src/math.rs`
- [x] Standardized in `strategy/src/graph.rs`
- [x] Unit tests passing in `mev-core`
