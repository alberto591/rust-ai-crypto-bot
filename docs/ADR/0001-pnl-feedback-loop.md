# ADR 0001: PnL Feedback Loop & Success Library

## Context
As the MEV bot matures, the need for data-driven strategy refinement became critical. Previously, trade outcomes were logged to flat files, making it difficult to correlate profit/loss with specific token "DNA" (early-stage metadata like launch hour, liquidity, and socials). To enable automated learning and potential ML-driven filtering, we need a robust, decentralized way to persist confirmed trade results into a structured library.

## Decision
We implemented a **PostgreSQL-backed Success Library** that captures the "DNA" of successful (and failed) trades. 

### Architectural Highlights:
1.  **Hexagonal Bridge**: Added `log_trade_landed` to the `TelemetryPort` and `save_story` to the `MarketIntelligencePort`. 
2.  **Decentralized Intelligence**: The `BotMetrics` component acts as the bridge. When a trade lands, it collects metadata from the `ArbitrageOpportunity` and asynchronously persists it to PostgreSQL via the `MarketIntelligence` adapter.
3.  **Data Enrichment**: The system now carries `initial_liquidity` and `launch_hour_utc` from the moment of discovery through to the database.
4.  **Fail-Safe Storage**: `MarketIntelligence` implements a file-system fallback (`library/success_*.json`) if the database is unreachable, ensuring no trade data is lost.

## Consequences
- **Positive**: Enables SQL-based analysis of what makes a "winning" token.
- **Positive**: Provides a structured dataset to train Future DNA models.
- **Negative**: Adds a dependency on PostgreSQL for full functionality.
- **Negative**: Slight increase in memory usage to carry metadata in `ArbitrageOpportunity`.

## Status
Accepted / Implemented (Phase 4-6)
