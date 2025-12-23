# ADR-015: Dev-to-Mainnet Testing & Safety Guardrails

**Status:** Accepted  
**Date:** 2025-12-23  
**Deciders:** Lead Engineer, Risk Management Team  

## Context

Moving from Devnet to Mainnet is the highest risk phase of HFT bot development. Devnet does not accurately replicate Mainnet liquidity, slippage, or congestion. To minimize the risk of "fat-finger" errors, front-running, or draining the wallet, we need a "Ghost Chain" environment and real-time safety guardrails.

## Decision

Adopt a multi-stage **Safe Launch Sequence** and implement hardcoded institutional-grade safety gates.

### 1. Ghost Chain Environment
Instead of testing on an empty local validator, we will use a "Ghost Chain" by cloning the critical Mainnet ecosystem using `solana-test-validator`.

**Command:**
```bash
solana-test-validator \
  --url https://api.mainnet-beta.solana.com \
  --reset \
  --clone 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2 \ # SOL/USDC Pool
  --clone 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 \ # Raydium V4
  --clone TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA \ # Token Program
  --clone EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \ # USDC Mint
  --clone So11111111111111111111111111111111111111112 \ # WSOL Mint
  --clone DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSb9JtteuvPpz \ # USDC Vault
  --clone HLmqeL62xR1QoZ1HKKbXRrdN1p3ph9EHDW6o7e6hEfWr   # WSOL Vault
```

### 2. Pre-Flight Simulation
The `ExecutionPort` must simulate transactions using the RPC's `simulateTransaction` method before broadcasting. 
- **Abort on Failure**: If simulation returns an error, the trade is terminated immediately.
- **Gas Verification**: We use simulation to estimate and log compute units consumed.

### 3. Institutional Safety Gates
The engine core will enforce hardcoded limits that take precedence over any strategy logic.
- **MAX_TRADE_SIZE**: 1.0 SOL (Panic limit).
- **MIN_PROFIT_THRESHOLD**: 5,000 lamports (Profitability floor after gas).

## Alternatives Considered

### Direct-to-Mainnet Burner Test
**Rejected**: Too risky. Even with 0.02 SOL, logic errors can result in account lockups or wasted gas during congestion.

### Devnet Testing Only
**Rejected**: Devnet market state is artificial. It cannot test regional liquidity or Raydium V4 pool depths accurately.

## Consequences

### Positive
- **Risk Mitigation**: "Ghost Chain" testing ensures the bot correctly handles real Mainnet account data.
- **Fail-Fast**: Simulation catches instruction building errors for free (no gas paid).
- **Safety**: Hardcoded limits prevent the bot from liquidating the entire wallet on a bug.

### Negative
- **Setup Complexity**: Requires the Solana CLI and manual cloning of vault addresses.
- **Latency**: Pre-flight simulation adds ~100-200ms to the execution path (only used in `LegacyExecutor`, not `JitoExecutor`).

## Related ADRs
- ADR-009: Simulation Mode
- ADR-011: Wallet Management
- ADR-014: Testing Strategy
