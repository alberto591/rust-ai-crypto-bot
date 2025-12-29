# Monitoring Guide - Optimized Bot

**Deployment**: Dec 29, 2024 16:34  
**PID**: 60394  
**Log**: `logs/optimized_run_20241229_163443.log`

---

## Quick Health Check

```bash
# Check if bot is running
ps aux | grep "60394" | grep -v grep

# View latest logs
tail -50 logs/optimized_run_*.log

# Monitor in real-time
tail -f logs/optimized_run_*.log
```

---

## Optimization Indicators

### 1. Rate Limiting (Expected)
```bash
# Look for throttling messages
tail -200 logs/optimized_run_*.log | grep "⏳ Hydration throttled"
```
**What to expect**: Frequent throttling messages indicating rate limiting is active

### 2. Pump.fun Deserialization (Fixed)
```bash
# Check success rate
tail -200 logs/optimized_run_*.log | grep "✅ \[Unified\] Hydrated Pump.fun"

# Check for failures (should be minimal)
tail -200 logs/optimized_run_*.log | grep "❌ Failed to deserialize curve"
```
**What to expect**: 
- Many successful hydrations
- Very few (if any) deserialization failures

### 3. Discovery Rate
```bash
# Count discoveries per minute
tail -1000 logs/optimized_run_*.log | grep "New Pool Detected" | wc -l
```
**What to expect**: Lower rate due to throttling

---

## Performance Metrics

### Credit Usage (Check after 24 hours)
1. Go to https://dashboard.helius.dev
2. Navigate to **Usage** → **Method Breakdown**
3. Compare:
   - `GET_TRANSACTION`: Should be ~260k/day (was 7.5M)
   - `GET_ACCOUNT_INFO`: Should be ~2.5M/day (was 5M)
   - `sendTransaction`: Should be 0/day if using Helius Sender

### Success Metrics
```bash
# Deserialization success rate
TOTAL=$(tail -1000 logs/optimized_run_*.log | grep -c "Found Pump.fun Bonding Curve")
SUCCESS=$(tail -1000 logs/optimized_run_*.log | grep -c "Hydrated Pump.fun Curve")
echo "Success rate: $((SUCCESS * 100 / TOTAL))%"
```
**Target**: >95% (was 69.5%)

---

## Troubleshooting

### Bot Crashed
```bash
# Check for errors
tail -100 logs/optimized_run_*.log | grep -E "(ERROR|panic|FATAL)"

# Restart
RUST_LOG=info,strategy=debug ./target/release/engine > logs/optimized_run_$(date +%Y%m%d_%H%M%S).log 2>&1 &
```

### Too Aggressive Throttling
If you see many missed opportunities:
```bash
# Edit watcher.rs and increase semaphore limit
# From: Semaphore::new(3)
# To: Semaphore::new(5) or Semaphore::new(10)

# Rebuild
cargo build --release
```

### High Credit Usage
If credits are still high after 24 hours:
1. Check Helius dashboard for method breakdown
2. Verify `HELIUS_SENDER_URL` is set correctly
3. Check logs for "Fallback transaction succeeded via Helius Sender"

---

## Expected Timeline

| Time | Expected Behavior |
|------|------------------|
| **0-1 hour** | Startup, initial discoveries, throttling begins |
| **1-6 hours** | Caches warming up, optimization stabilizing |
| **6-24 hours** | Full optimization active |
| **24+ hours** | Verify credit reduction on dashboard |

---

## Key Log Messages

✅ **Good Signs**:
- `⏳ Hydration throttled` - Rate limiting working
- `✅ [Unified] Hydrated Pump.fun Curve` - Deserialization successful
- `Fallback transaction succeeded via Helius Sender` - 0-credit transactions

⚠️ **Warning Signs**:
- Many `❌ Failed to deserialize curve` - Deserialization still failing
- No throttling messages - Rate limiting not working
- High CPU usage - Potential issue

---

## Next Steps

1. **Monitor for 1 hour**: Verify optimizations are active
2. **Check after 24 hours**: Verify credit reduction on Helius dashboard
3. **Document results**: Update walkthrough with actual metrics
4. **Fine-tune if needed**: Adjust semaphore limit or thresholds

**Target**: <3M credits/day (currently projected 2.5M)
