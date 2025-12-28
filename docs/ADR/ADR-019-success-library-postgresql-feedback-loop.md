# ADR-019: Success Library Infrastructure & Autonomous Feedback Loop

**Status:** Accepted
**Date:** 2025-12-27
**Author:** Antigravity

## 1. Context (The "Why")
The HFT bot requires a persistent memory to "learn" from both successful token launches and "false positives" (rugs/honeypots). The initial Phase 2 file-based storage was insufficient for real-time querying and high-level pattern analysis. We needed a robust, high-performance database layer that could be queried by the strategy engine mid-trade to prevent repeating mistakes.

## 2. Decision
We have transitioned the "Success Library" to a PostgreSQL-backed infrastructure with an integrated feedback loop:
1.  **Storage**: Standardized on PostgreSQL using `tokio-postgres` and `deadpool-postgres` (connection pooling).
2.  **Hybrid Persistence**: Implemented a fallback mechanism where the bot automatically reverts to JSON file storage if the database is unreachable, ensuring zero data loss during ingestion.
3.  **Hexagonal Inversion**: Created the `MarketIntelligencePort` in the `strategy` crate and implemented it via `DatabaseIntelligence` in the `engine` crate, maintaining strict layer isolation.
4.  **Feedback Loop**: Wired the `StrategyEngine` to check the Success Library for blacklisted (False Positive) tokens before every execution.

## 3. Rationale (The "Proof")
*   **Dependency Stability**: `tokio-postgres` was selected to avoid the version conflicts with the Solana 1.17 SDK (specifically `zeroize` and `spl-token`) that were encountered when testing `sqlx`.
*   **Performance**: `deadpool` connection pooling ensures low-latency reads during the "Birth Watching" phase of token discovery.
*   **Scalability**: SQL-based analysis (`--analyze` mode) allows for complex DNA extraction (Average ROI, Mean Time to Peak) that would be computationally expensive with flat JSON files.

## 4. Consequences
*   **Positive:** Institutional-grade data persistence; automatic learning feedback loop; reduced exposure to known bad actors.
*   **Negative/Trade-offs:** Introduces a dependency on a running PostgreSQL instance for the full feature set.

## 5. Wiring Check (No Dead Code)
- [x] Schema defined in [init_db.sql](file:///Users/lycanbeats/Desktop/Rust%20AI%20Chatbox/scripts/init_db.sql)
- [x] Logic implemented in [intelligence.rs](file:///Users/lycanbeats/Desktop/Rust%20AI%20Chatbox/engine/src/intelligence.rs)
- [x] Feedback loop wired in [strategy/lib.rs](file:///Users/lycanbeats/Desktop/Rust%20AI%20Chatbox/strategy/src/lib.rs)
- [x] Multi-mode orchestration in [main.rs](file:///Users/lycanbeats/Desktop/Rust%20AI%20Chatbox/engine/src/main.rs)
- [x] `is_false_positive` field added to `SuccessStory` in [core/lib.rs](file:///Users/lycanbeats/Desktop/Rust%20AI%20Chatbox/core/src/lib.rs)
