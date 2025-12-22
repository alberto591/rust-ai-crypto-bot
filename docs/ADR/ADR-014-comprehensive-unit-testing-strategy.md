# ADR-014: Comprehensive Unit Testing Strategy

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

High-frequency trading engines operate on thin margins and high risk. Logic errors in math, configuration, or instruction building can lead to significant financial loss. A robust verification layer is required beyond just "integration testing" or "simulation mode."

## Decision

Adopt a **Comprehensive Unit Testing Strategy** targeting the core logic of every crate (`core`, `strategy`, `executor`, `engine`).

- **Mathematical Correctness**: Unit tests for CPMM constant-product math and price impact calculations.
- **Instruction Validation**: Verify the byte-layout of manually constructed Raydium and Orca instructions.
- **Configuration Integrity**: Test environment variable parsing and multi-source configuration merging.
- **Wallet Logic Verification**: Test the generation of ATA and WSOL instructions without live RPC connections.

## Requirements

1. **Deterministic Tests**: No reliance on live Mainnet RPCs for unit tests.
2. **Clippy Enforcement**: All crates must pass `cargo clippy --workspace -- -D warnings` to ensure idiomatic and safe code.
3. **Workspace-wide Execution**: Ensure `cargo test --workspace` covers 100% of critical execution paths.

## Alternatives Considered

### Live Integration Testing Only
**Rejected**: Too slow, expensive (requires Mainnet transactions), and doesn't catch edge-case math bugs.

### No Math Testing (Trust Crate)
**Rejected**: We use custom high-perf pathmath; it must be verified against known Solana program behavior.

### Comprehensive Unit Testing (CHOSEN)
**Pros**:
- High developer confidence.
- Rapid regression detection during refactoring.
- Zero-cost verification (doesn't require SOL or RPCs).

**Cons**:
- Increased development time for new features.
- Maintenance overhead for test suites.

## Consequences

### Positive
- Rock-solid "Brain" (Strategy) and "Hands" (Executor).
- Clean code as enforced by Clippy.
- Easy to onboard new developers/strategies safely.

### Negative
- Slightly more verbose codebase due to `#[cfg(test)]` modules in every file.

## Related ADRs
- ADR-009: Simulation Mode (Complements unit tests with dynamic verification)
- ADR-011, ADR-012, ADR-013: All verified via this strategy.
