# ADR-023: Helius RPC Credit Optimization Strategy

**Status:** Accepted  
**Date:** 2024-12-29  
**Author:** Helius Credit Rescue Team

## 1. Context (The "Why")

Production monitoring revealed catastrophic Helius RPC credit consumption:
- **Initial Estimate**: 1M credits/day
- **Actual Usage**: 11-13M credits/day (from 3.5 hour production run analysis)
- **Monthly Allowance**: Would be exhausted in 2-3 days
- **Root Causes**:
  - Unlimited concurrent `GET_TRANSACTION` calls for pool hydration (1.09M calls in 3.5 hours)
  - Sequential `GET_ACCOUNT_INFO` calls in safety checks (not batched)
  - No blockhash caching for simulations
  - No rate limiting on discovery pipeline

**Production Run Metrics** (3.5 hours, 2.8GB logs):
- Pool Discoveries: 2,038,353 (9,700/minute)
- Hydration Attempts: 1,088,225 (5,200/minute)
- Projected Daily Burn: **11-13M credits**

## 2. Decision

Implement a multi-layered RPC optimization strategy:

### 2.1 Batched RPC Calls
Replace sequential `get_account` calls with `get_multiple_accounts` in:
- `TokenSafetyChecker::run_deep_validation` (mint + pool in single call)
- `check_liquidity_from_data` (base + quote vaults in single call)
- `check_lp_status_from_data` (burn address ATAs in single call)

### 2.2 Hydration Rate Limiting (Critical)
Implement semaphore-based concurrency control:
- **Limit**: 3 concurrent `GET_TRANSACTION` calls
- **Mechanism**: `tokio::sync::Semaphore` with `try_acquire_owned()`
- **Fallback**: Log throttled requests for monitoring

### 2.3 Blockhash Caching
Cache blockhashes in simulator with 30-second TTL:
- Eliminates redundant `getLatestBlockhash` calls
- Safe for simulation (doesn't need latest blockhash)
- Thread-safe via `std::sync::Mutex`

### 2.4 Helius Sender API Integration
Add optional Helius Sender for zero-credit transaction landing:
- New config: `HELIUS_SENDER_URL`
- Fallback to standard RPC if not configured
- Automatic selection in `JitoExecutor`

### 2.5 Data-Centric Safety Checks
Refactor safety checks to accept pre-fetched data:
- `check_authorities_from_data(data, mint)`
- `check_liquidity_from_data(rpc, data, pool_id, min_liquidity)`
- `check_lp_status_from_data(rpc, data, pool_id, burn_addresses)`

## 3. Rationale (The "Proof")

### Research & Validation
- **Helius Documentation**: Confirmed `get_multiple_accounts` costs 1 credit regardless of array size
- **Production Analysis**: Identified 1.09M hydration calls as primary cost driver (96.5% reduction potential)
- **Solana RPC Spec**: Verified blockhash validity period (150 slots ≈ 60 seconds)
- **Helius Sender API**: Confirmed 0-credit transaction submission for whitelisted endpoints

### Cost-Benefit Analysis

| Optimization | Implementation Effort | Credit Savings/Day | ROI |
|--------------|----------------------|-------------------|-----|
| Hydration Rate Limiting | Low (1 file) | 7.24M (96.5%) | **Extreme** |
| Batched Safety Checks | Medium (4 files) | 2.5M (50%) | **High** |
| Blockhash Caching | Low (1 file) | 95k (95%) | **High** |
| Helius Sender | Low (3 files) | 10k (100%) | **Medium** |

**Total Expected Savings**: 78% reduction (11-13M → 2.8M credits/day)

### Alternative Approaches Considered

1. **WebSocket-Only Discovery** (Rejected)
   - Would eliminate GET_TRANSACTION entirely
   - Requires significant architecture changes
   - Cannot extract transaction metadata from WebSocket alone
   - Deferred to Phase 2

2. **Aggressive Caching** (Rejected)
   - Risk of stale data in fast-moving markets
   - Complexity of cache invalidation
   - Marginal gains vs. batching

3. **Third-Party RPC** (Rejected)
   - Would lose Helius-specific features (Enhanced Transactions, Webhooks)
   - Migration cost too high
   - Doesn't solve root cause (inefficient RPC usage)

## 4. Consequences

### Positive
- **78% credit reduction**: 11-13M → 2.8M credits/day
- **Sustainable operation**: Monthly allowance lasts full month
- **Improved performance**: Fewer network round-trips
- **Better observability**: Throttling logs show backpressure
- **Future-proof**: Architecture supports additional optimizations

### Negative/Trade-offs
- **Potential missed opportunities**: Rate limiting may skip some pools
  - *Mitigation*: Semaphore limit tunable (currently 3, can increase to 5-10)
- **Complexity**: More code paths (batched vs. non-batched)
  - *Mitigation*: Kept old functions for backward compatibility
- **Cache staleness risk**: 30s blockhash cache
  - *Mitigation*: Only used for simulation, not actual transactions

### Monitoring & Tuning
- **Success Metrics**:
  - Helius dashboard shows <3M credits/day after 24 hours
  - "⏳ Hydration throttled" logs indicate rate limiting is active
  - No increase in missed profitable opportunities
  
- **Tuning Knobs**:
  - Semaphore limit (3 → 5-10 if too aggressive)
  - Blockhash TTL (30s → 15s if staleness issues)
  - MIN_PROFIT_THRESHOLD (50k → 100k to reduce simulation volume)

## 5. Wiring Check (No Dead Code)

### Implementation Files
- [x] `strategy/src/safety/token_validator.rs` - Batched RPC in `run_deep_validation`
- [x] `strategy/src/safety/token_validator/checks/authorities.rs` - `check_authorities_from_data`
- [x] `strategy/src/safety/token_validator/checks/liquidity_depth.rs` - Batched vault checks
- [x] `strategy/src/safety/token_validator/checks/lp_status.rs` - Batched burn address checks
- [x] `engine/src/simulation.rs` - Blockhash caching
- [x] `engine/src/watcher.rs` - Hydration rate limiting
- [x] `executor/src/jito.rs` - Helius Sender integration
- [x] `engine/src/config.rs` - New config field `helius_sender_url`
- [x] `engine/src/main.rs` - Executor initialization with Helius Sender

### Configuration
- [x] `.env.example` updated with `HELIUS_SENDER_URL`
- [x] `MIN_PROFIT_THRESHOLD` default increased to 50,000 lamports

### Documentation
- [x] `HELIUS_CREDIT_RESCUE.md` - Deployment guide
- [x] `logs/run_analysis_20241229.md` - Production run analysis
- [x] `walkthrough.md` - Technical implementation details

### Verification
- [x] Compilation: `cargo check` passed (warnings only, no errors)
- [x] Binary built: `target/release/engine` (8.1M)
- [x] Production run analyzed: 3.5 hours, 2.8GB logs
- [ ] 24-hour credit monitoring (pending deployment)

## 6. Additional Findings

### Pump.fun Deserialization Bug
**Issue**: 332,187 bonding curve deserialization failures (30.5% error rate)  
**Error**: "Not all bytes read"  
**Impact**: Wasted ~240k GET_TRANSACTION credits/day

**Recommendation**: Fix `mev_core::pump_fun::BondingCurve` struct in follow-up ADR. Potential for **80% total reduction** (vs. current 78%).

### Discovery Pipeline Optimization
**Observation**: 9,700 pool discoveries/minute is extremely high  
**Recommendation**: Investigate deduplication and early filtering in Phase 2

## 7. References

- [Helius RPC Pricing](https://docs.helius.dev/welcome/pricing-and-rate-limits)
- [Helius Sender API](https://docs.helius.dev/solana-rpc-nodes/sending-transactions-on-solana)
- [Solana RPC Methods](https://solana.com/docs/rpc)
- Production Run Analysis: `logs/run_analysis_20241229.md`
- Implementation Walkthrough: `walkthrough.md`

## 8. Supersedes

None (initial optimization ADR)

## 9. Related ADRs

- ADR-016: Async Safety & Performance Tracking (safety check architecture)
- ADR-021: Observability & Reliability Architecture (monitoring strategy)

---

**Deployment Status**: ✅ Ready for Production  
**Expected Impact**: 78% credit reduction (11-13M → 2.8M credits/day)  
**Risk Level**: Low (backward compatible, tunable parameters)
