# Devnet Dry Run - Quick Start

## 🚀 Quick Setup (5 minutes)

### 1. Get Devnet SOL
```bash
# Set to devnet
solana config set --url devnet

# Airdrop SOL for testing
solana airdrop 2

# Check balance
solana balance
```

### 2. Configure Environment
```bash
# Copy example config
cp .env.example .env

# Edit .env - set these values:
RPC_URL=https://api.devnet.solana.com
KEYPAIR_PATH=~/.config/solana/id.json
DRY_RUN=true
```

### 3. Run the Test
```bash
cargo run --release --bin engine
```

## Expected Results

### ✅ Success Case A: Transaction Submitted
```
🎉 SUCCESS! Transaction submitted!
📝 Signature: 2kX9...
🔍 Explorer: https://explorer.solana.com/tx/2kX9...?cluster=devnet
```
**This means:** Everything works perfectly!

### ✅ Success Case B: Expected Failure  
```
✅ SUCCESS (Expected Failure)

The transaction was REJECTED by the validator because
the placeholder pool accounts don't exist. This is GOOD!

What this proves:
  ✅ Instruction builder works correctly
  ✅ Transaction signing works
  ✅ RPC communication works
  ✅ The transaction reached Solana validators
```
**This means:** The engine is working! The rejection proves the instruction reached the blockchain.

### ❌ Actual Failure
```
❌ FAILED with unexpected error:
Insufficient funds

Common issues:
  - Insufficient SOL (run: solana airdrop 2 --url devnet)
  - Network connectivity
  - RPC rate limiting
```
**Fix:** Follow the suggested solutions.

## What Gets Tested

The dry run tests:
- ✅ Raydium V4 instruction builder (`executor/raydium_builder.rs`)
- ✅ Legacy RPC executor (`executor/legacy.rs`)
- ✅ Transaction signing
- ✅ Network communication
- ✅ Devnet connectivity

## Limitations

This test uses **placeholder pool accounts**, so the swap won't actually execute. That's intentional! We're testing the builder, not making real swaps.

For real swaps, you'd need:
- Actual Raydium pool account addresses
- Wrapped SOL and USDC token accounts
- Proper ATA (Associated Token Account) setup

## Next Steps

After verifying the engine works:
1. Wait for Jito connectivity
2. Add real pool data from `MarketGraph`
3. Implement proper account lookups
4. Add slippage protection
5. Enable production mode

## Troubleshooting

### "Failed to load keypair"
```bash
# Generate a new keypair
solana-keygen new
```

### "Insufficient SOL"
```bash
# Get more SOL
solana airdrop 2 --url devnet
```

### "Connection refused"
Check your internet connection or try a different RPC:
```bash
# Alternative devnet RPCs
RPC_URL=https://api.devnet.solana.com
RPC_URL=https://devnet.rpcpool.com
```

## Full Documentation

See `docs/DEVNET_DRY_RUN.md` for detailed setup with real pool addresses.
