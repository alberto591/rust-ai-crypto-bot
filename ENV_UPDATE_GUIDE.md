# Environment Configuration Update

## Required Changes to `.env`

### 1. Add Helius Sender URL (Optional but Recommended)
Add this line to enable 0-credit transaction landing:
```bash
HELIUS_SENDER_URL=https://mainnet.helius-rpc.com/?api-key=a3b90fe6-fbba-48fa-abfb-34eaf8a61e85
```

### 2. Update MIN_PROFIT_THRESHOLD
Change from `20000` to `50000` to reduce simulation volume:
```bash
# Before
MIN_PROFIT_THRESHOLD=20000

# After
MIN_PROFIT_THRESHOLD=50000
```

---

## Quick Update Commands

```bash
# Add Helius Sender URL
echo "HELIUS_SENDER_URL=https://mainnet.helius-rpc.com/?api-key=a3b90fe6-fbba-48fa-abfb-34eaf8a61e85" >> .env

# Update MIN_PROFIT_THRESHOLD
sed -i '' 's/MIN_PROFIT_THRESHOLD=20000/MIN_PROFIT_THRESHOLD=50000/' .env
```

---

## Deployment

Once `.env` is updated:

```bash
# Stop current bot
pkill -f "target/release/engine"

# Start optimized binary (background)
pkill -f "target/release/engine"
RUST_LOG=info,strategy=debug ./target/release/engine --no-tui > logs/optimized_run_$(date +%Y%m%d_%H%M%S).log 2>&1 &

# Or with TUI (foreground only)
RUST_LOG=info,strategy=debug ./target/release/engine
```

---

## Verification

After 10 minutes, check logs:
```bash
# Look for rate limiting
tail -100 logs/optimized_run_*.log | grep "⏳ Hydration throttled"

# Verify Pump.fun success
tail -100 logs/optimized_run_*.log | grep "✅ \[Unified\] Hydrated Pump.fun"

# Check for deserialization errors (should be minimal)
tail -100 logs/optimized_run_*.log | grep "❌ Failed to deserialize curve"
```

---

**Current RPC URL**: Already configured with Helius API key ✅  
**Optimized Binary**: `target/release/engine` (8.1M, built Dec 29 16:07) ✅  
**Expected Impact**: 80% credit reduction (11-13M → 2.5M/day)
