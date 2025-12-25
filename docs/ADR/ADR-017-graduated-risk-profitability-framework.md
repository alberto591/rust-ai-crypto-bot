# ADR-017: Graduated Risk Profitability Framework

## Status
Accepted

## Context
In high-frequency trading (HFT) and Solana MEV, there is no absolute guarantee of profit. To mitigate financial risk while validating the bot's efficacy, we need a rigorous framework to determine if returns will exceed the operational costs (specifically the $49-$99/month RPC costs) before committing significant capital.

We have identified two success archetypes:
1. **The "Tammer" Benchmark**: Logic-centric. Using probability and sizing (Kelly Criterion) to maximize returns on "shitcoins" after ATHs.
2. **The "Immy" Benchmark**: Logistics-centric. Focusing on superior infrastructure (bare metal, global distribution) to outpace the network.

## Decision
We will adopt a **Graduated Risk Approach** to verify profitability. This framework mandates a specific sequence of validation steps, moving from zero-risk simulation to low-risk micro-trading, only scaling when specific mathematical proofs of "edge" are met.

### The "ROI of the Data Pipe" Math
To cover a Helius Developer Plan ($49/month) or Pro Plan ($99/month) and clear a $50 profit:
- **Daily Target**: $3.30 - $5.00
- **Rationale**: This is a conservative floor compared to benchmarks ($4,000/day for "Shitty Bot").

## Implementation: The Verification Sequence

### Phase 1: The "Data-Only" Week (0% Capital Risk)
**Goal**: Verify statistical edge without risking funds.
- **Prerequisite**: Paid RPC key.
- **Configuration**: `EXECUTION_MODE=Simulation`.
- **Action**: The bot "listens" to the real Raydium firehose and simulates potential trades.
- **Evidence**: `data/arbitrage_data.csv` (or trade log).
- **Verification Condition**: After 48 hours, simulated profit > $10 (2x daily target).

### Phase 2: The "Calibration" Check
**Goal**: Verify infrastructure stability on stable pairs.
- **Scope**: SOL/USDC pair only for first 24 hours.
- **Action**: Monitor `Pools` count and `PerformanceTracker`.
- **Insight**: Lack of opportunities here indicates **latency** issues (infrastructure), not code issues.

### Phase 3: The "Rug Filter" Validation
**Goal**: Verify safety vs. volatility.
- **Scope**: Volatile pairs (where "Immy" and "Tammer" profits are).
- **Metric**: Track "New Pools" detected vs. "Rejected" by `TokenSafetyChecker`.
- **Verification Condition**: Rejection of a pool that later rugs is valued at >$50 (cost saved).

## Strategic Direction
We will combine the archetypes:
- **Logic ("Tammer")**: Use Kelly Criterion for position sizing as win rate is proven.
- **Logistics ("Immy")**: The paid RPC is the first step toward the required infrastructure speed.

## The "Enough Thinking" Checklist (Exit Criteria)
1. **Commit**: Treat the RPC cost ($49-99) as a 30-day R&D expense.
2. **Simulate**: Run Simulation Mode for 3 Days.
   - **Fail Safe**: If simulated profit < $15 total, cancel subscription.
3. **Micro-Trade**: Land 10 real trades at 0.01 SOL (approx $1-2) to verify execution.
4. **Scale**: Only after step 3 is confirmed.

This framework replaces "hope" with log-based verification.
