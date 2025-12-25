# Production Runbook: Solana MEV Bot

## Pre-Flight Checklist
- [ ] `.env` configured with `EXECUTION_MODE=LiveProduction`
- [ ] Keypair has ≥1 SOL for gas
- [ ] Discord/Telegram webhooks configured
- [ ] Grafana dashboard imported

## Starting Production
```bash
# 1. Build
cargo build --release

# 2. Launch
./scripts/launch_tui.sh

# 3. Verify TUI shows "LiveProduction" mode
```

## Monitoring

### Key Metrics (http://localhost:9090/metrics)
- `daily_pnl_lamports` - Daily profit/loss
- `circuit_breaker_triggers` - Risk limit hits
- `safety_rejections` - Rejected opportunities

### Capital Scaling Tiers
| Tier | Max | Requirement |
|------|-----|-------------|
| 1 | 0.01 SOL | Initial |
| 2 | 0.05 SOL | 70%+ win, 100 trades |
| 3 | 0.1 SOL | 70%+ win, 200 trades |
| 4 | 0.5 SOL | 75%+ win, 500 trades |

## Emergency Procedures

### Circuit Breaker Triggered
- **Cause**: 5 consecutive losses
- **Action**: Restart bot (auto-resets after 24h)

### High Slippage
- **Action**: Reduce `MAX_SLIPPAGE_BPS` in `.env`

### RPC Failures
- **Action**: Add backup URLs to `.env`:
```bash
RPC_URL_BACKUP=https://api.mainnet-beta.solana.com
```

## Emergency Shutdown
```bash
# Graceful
pkill -15 engine

# Force
pkill -9 engine
```

## Daily Operations
- Check overnight P&L
- Review error logs
- Verify SOL balance
