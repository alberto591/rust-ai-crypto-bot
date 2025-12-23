# ADR [016]: Async Safety Checks and Thread-Safe Performance Tracking

**Status:** Accepted
**Date:** 2025-12-23
**Author:** Antigravity

## 1. Context (The "Why")
The current implementation lacks robust safety checks for Liquidity Provider (LP) locks, making the bot vulnerable to "rug pulls". Additionally, performance tracking (logging) is currently synchronous, which can block the main trading event loop and increase latency in high-frequency trading (HFT) scenarios. Configuration validation is also minimal, leading to potential silent failures if environment variables are misconfigured.

## 2. Decision
We are implementing three critical improvements:
1. **Thread-Safe Logging:** A non-blocking `PerformanceTracker` using an async MPSC channel to handle file I/O in a background task.
2. **Async Safety Checks:** A fully async `TokenSafetyChecker` that verifies token authorities, holder distribution, and LP lock status (the "Rug Shield").
3. **Validated Config:** A robust `BotConfig` loader that validates `EXECUTION_MODE` and prevents dangerous configurations (e.g., using Public RPC for live trading).

## 3. Rationale (The "Proof")
* **Non-blocking I/O:** Using `tokio::sync::mpsc` ensures that logging trades does not add latency to the execution path.
* **Rug Shield:** Specifically checking for LP burns or locks in known addresses (like System Program or Dead addresses) significantly reduces risk.
* **Fail-Fast Config:** Validating settings at startup ensures the bot operates within safety limits and avoids "Undefined Behavior".
* **Institutional Grade:** These patterns are standard in professional trading systems to ensure reliability and speed.

## 4. Consequences
* **Positive:** Reduced latency in the main loop, improved protection against scams, and more reliable bot initialization.
* **Negative/Trade-offs:** Slighly higher memory usage due to the log buffer; potential log drops under extreme congestion (to prioritize trading).

## 5. Wiring Check (No Dead Code)
- [ ] `PerformanceTracker` implemented in `engine/src/analytics/performance.rs`
- [ ] `TokenSafetyChecker` implemented in `engine/src/safety/token_validator.rs`
- [ ] `BotConfig` updated in `engine/src/config.rs`
- [ ] Safety limits added to `.env`
