# ADR-022: Orca Whirlpool CLMM Integration

**Status:** Accepted
**Date:** 2025-12-28
**Author:** Antigravity

## 1. Context (The "Why")
Standard CPMM (Raydium V4) pools provide a baseline for arbitrage, but the majority of high-alpha volume on Solana has shifted to Concentrated Liquidity Market Makers (CLMM) like Orca Whirlpools. To remain competitive and access deeper liquidity with lower price impact, the bot must support CLMM account decoding and instruction building.

## 2. Decision
We are implementing native support for Orca Whirlpools across the stack:
* **Core**: Added `Whirlpool` account decoder in `core/src/orca.rs`.
* **Discovery**: Added log-based discovery for `InitializePool` events.
* **Executor**: Implemented `Whirlpool` swap instruction building and TickArray PDA derivation.
* **Strategy**: Integrated Orca pools into the `MarketGraph` alongside Raydium.

## 3. Rationale (The "Proof")
* Orca Whirlpools allow for more capital-efficient trades.
* Cross-DEX arbitrage between Raydium (CPMM) and Orca (CLMM) is a major profit source.
* Formalized account decoding ensures we don't rely on slow RPC `get_amount_out` calls.

## 4. Consequences
* **Positive:** Access to broader arbitrage surface; improved capital efficiency.
* **Negative/Trade-offs:** Significant increase in codebase complexity (CLMM math is harder than CPMM).

## 5. Wiring Check (No Dead Code)
- [x] Logic implemented in `core/src/orca.rs`
- [x] Discovery logic in `engine/src/discovery.rs`
- [x] Builder in `executor/src/orca_builder.rs`
- [x] Integrated in `strategy/src/graph.rs`
