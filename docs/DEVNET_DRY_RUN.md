# Devnet Dry Run Setup Guide

## Prerequisites

1. **Install Solana CLI** (if not already installed):
```bash
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
```

2. **Get Devnet SOL**:
```bash
# Create or use existing devnet wallet
solana-keygen new --outfile ~/.config/solana/devnet.json

# Set to devnet
solana config set --url devnet

# Airdrop SOL
solana airdrop 2

# Check balance
solana balance
```

## Finding Raydium Devnet Pool Addresses

### Option 1: Raydium API
```bash
curl "https://api.raydium.io/v2/ammV3/ammPools" | jq '.data.Devnet'
```

### Option 2: Manual Lookup
Visit Raydium's devnet interface or check their GitHub for deployed pool addresses.

### Option 3: Query On-Chain
```bash
# Get program accounts for Raydium V4 on devnet
solana program show 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8 --url devnet
```

## Steps to Run

### 1. Create Token Accounts

You need wrapped SOL and USDC token accounts:

```bash
# Create Associated Token Accounts
spl-token create-account So11111111111111111111111111111111111111112 --url devnet  # Wrapped SOL
spl-token create-account EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --url devnet  # USDC (devnet mint)

# Wrap some SOL
spl-token wrap 1 --url devnet
```

### 2. Update Pool Addresses

Edit `examples/devnet_dry_run.rs` and replace placeholder addresses:
- `DEVNET_AMM_POOL_ADDRESS_HERE`  
- `DEVNET_AMM_AUTHORITY_HERE`
- etc.

Also update:
- `YOUR_SOL_TOKEN_ACCOUNT` - Your wrapped SOL ATA
- `YOUR_USDC_TOKEN_ACCOUNT` - Your USDC ATA

### 3. Run the Test

```bash
cargo run --example devnet_dry_run --release
```

### 4. Verify on Solscan

Copy the transaction signature and paste it into:
```
https://solscan.io/tx/SIGNATURE_HERE?cluster=devnet
```

## Expected Output

```
🚀 Devnet Dry Run: Testing Raydium Swap Builder
================================================

💰 Wallet: ABC123...
🌐 Connected to Devnet RPC

📦 Pool Configuration:
   AMM: XYZ789...
   Source: USER_SOL_ACCOUNT
   Dest: USER_USDC_ACCOUNT

💱 Building Swap Instruction:
   Amount In: 100000000 lamports (0.1 SOL)
   Min Out: 0 (no slippage protection)

🚀 Executing transaction...

✅ SUCCESS!
📝 Transaction Signature: 2kX9...
🔍 View on Solscan: https://solscan.io/tx/2kX9...?cluster=devnet
```

## Troubleshooting

### "Insufficient funds"
```bash
solana airdrop 2 --url devnet
```

### "Account not found"
You need to create token accounts first (see step 1 above)

### "Invalid account data"
One of the pool addresses is incorrect or the pool doesn't exist on devnet

### "Transaction simulation failed"
- Check that wrapped SOL account has balance
- Verify all 18 accounts in RaydiumSwapKeys are correct
- Ensure slippage tolerance is set appropriately

## Production Checklist

Before using on mainnet:
- [ ] Use actual pool addresses from Raydium API
- [ ] Set proper slippage protection (`min_amount_out`)
- [ ] Add simulation before sending  
- [ ] Implement proper error handling
- [ ] Use secure keypair management
- [ ] Add transaction retry logic
- [ ] Monitor for failed transactions
