# ADR-011: Automated Wallet & Asset Management

**Status:** Accepted  
**Date:** 2025-12-22  
**Deciders:** Lead Rust Engineer  

## Context

Solana arbitrage requires precise management of Associated Token Accounts (ATAs) and Wrapped SOL (WSOL). Manual management of these accounts during high-frequency execution is error-prone and can lead to failed transactions (AccountNotFound) or trapped liquidity. 

## Decision

Implement an automated `WalletManager` in the `engine` crate to handle asset logistics programmatically.

- **ATA Auto-Creation**: Check for existence of required ATAs at startup and create them if missing.
- **Atomic WSOL Synchronization**: Wrap native SOL into WSOL only when needed to maintain liquidity.
- **Zero-residue Unwrapping**: Provide mechanisms to unwrap WSOL back to native SOL to avoid fragmented balances.

## Architecture

```rust
pub struct WalletManager {
    rpc: RpcClient,
}

impl WalletManager {
    pub fn ensure_ata_exists(&self, payer: &Pubkey, token_mint: &Pubkey) -> Option<Instruction>;
    pub fn sync_wsol(&self, payer: &Keypair, amount_lamports: u64) -> Result<Vec<Instruction>, Box<dyn Error>>;
    pub fn unwrap_wsol(&self, payer: &Pubkey) -> Result<Instruction, Box<dyn Error>>;
}
```

## Alternatives Considered

### Manual CLI Management
**Rejected**: Too slow for dynamic operation; risk of missing accounts during high-speed execution.

### Jito-specific Asset Handling
**Rejected**: Wallet management should be independent of the execution venue (Jito vs. Legacy RPC).

### WalletManager (CHOSEN)
**Pros**:
- Proactive readiness: Bot is always ready to trade.
- Reduced fragmentation: Automated wrapping/unwrapping keeps SOL balance clean.
- Error mitigation: Eliminates "AccountNotFound" errors for ATAs.

**Cons**:
- Slightly increased startup time.
- Rent costs for ATAs (one-time fee).

## Consequences

### Positive
- High reliability for trades involving new token pairs.
- Automated liquidity management (SOL <-> WSOL).
- Cleaner wallet overview for the user.

### Negative
- Requires extra SOL for rent-exemption of newly created accounts.

## Related ADRs
- ADR-001: Hexagonal Architecture (Infrastructure layer)
- ADR-013: Lazy Pool Key Fetching (Integration with ATA lookup)
