# ADR-020: MEV Bot Operations & Usage Guide

**Status:** Accepted  
**Date:** 2025-12-27  
**Author:** Antigravity

## 1. Context (The "Why")
The MEV bot is now production-ready with the Success Library integrated. We need clear operational procedures for deployment, monitoring, and maintenance to ensure reliable 24/7 operation.

## 2. Operating Modes

### Mode 1: Production Trading (Default)
**Purpose**: Active arbitrage trading on Solana mainnet with feedback loop

**Command**:
```bash
./target/release/engine
```

**Behavior**:
- Monitors real-time market updates via WebSocket
- Identifies arbitrage opportunities (up to 5-hop routes)
- Checks Success Library blacklist before each trade
- Executes profitable trades via Jito bundles
- Records metrics to Prometheus (port 8081)

**When to use**: 24/7 production operation

---

### Mode 2: Discovery Mode
**Purpose**: Ingest new token launches into Success Library

**Command**:
```bash
./target/release/engine --discovery
```

**Behavior**:
- Listens for new pool creation events (Raydium + Pump.fun)
- Tracks token performance from birth
- Stores "success stories" in PostgreSQL
- Runs concurrently with normal trading

**When to use**: Run continuously to build historical data

---

### Mode 3: Analysis Mode
**Purpose**: Generate "Success DNA" reports

**Command**:
```bash
./target/release/engine --analyze
```

**Behavior**:
- Queries PostgreSQL for aggregate metrics
- Displays: Avg ROI, Median time-to-peak, Total stories, Strategy effectiveness
- Exits after report

**When to use**: Weekly analysis or before strategy adjustments

---

### Mode 4: Combined Mode
**Purpose**: Full autonomous operation

**Command**:
```bash
./target/release/engine --discovery --analyze
```

**Behavior**: Trading + Discovery + Initial analysis report

**When to use**: Recommended for production deployment

---

## 3. Deployment Procedures

### Local Development
```bash
# 1. Set environment variables
cp .env.example .env
# Edit .env with your keys

# 2. Start local PostgreSQL
brew services start postgresql@17

# 3. Initialize database
psql -d mev_bot_success_library -f scripts/init_db.sql

# 4. Run bot
cargo run --release
```

### Production (Vultr)
```bash
# 1. SSH to server
ssh -i ~/.ssh/vultr_mev_bot root@149.28.35.68

# 2. Check service status
systemctl status mev-bot

# 3. View logs
journalctl -u mev-bot -f

# 4. Restart service
systemctl restart mev-bot
```

---

## 4. Monitoring & Observability

### Prometheus Metrics
**Endpoint**: `http://149.28.35.68:8080/metrics`

**Key Metrics**:
- `opportunities_found_total` - Total arbitrage opportunities detected
- `trades_executed_total` - Successful trade executions
- `pnl_sol_total` - Cumulative profit/loss in SOL
- `success_library_total` - Total success stories
- `success_library_blacklisted` - Blacklisted token count

### Log Locations
- **Systemd logs**: `journalctl -u mev-bot`
- **File logs**: `/opt/mev-bot/data/` (if enabled)

### Health Checks
```bash
# Bot is running
systemctl is-active mev-bot

# Database is accessible
psql -d mev_bot_success_library -c "SELECT COUNT(*) FROM success_stories;"

# Metrics endpoint responding
curl http://localhost:8080/metrics | head
```

---

## 5. Success Library Management

### CLI Commands
```bash
cd /opt/mev-bot

# View statistics
./scripts/manage_library.sh stats

# List blacklisted tokens
./scripts/manage_library.sh list-blacklist

# Add token to blacklist
./scripts/manage_library.sh add-blacklist <token_address>

# Remove from blacklist
./scripts/manage_library.sh remove-blacklist <token_address>

# Auto-detect false positives (ROI < -80%)
./scripts/manage_library.sh auto-detect -80

# Run DNA analysis
./scripts/manage_library.sh analyze
```

### Database Access
```bash
# Direct SQL access
sudo -u postgres psql -d mev_bot_success_library

# Export data
sudo -u postgres pg_dump mev_bot_success_library > backup.sql

# View recent stories
psql -d mev_bot_success_library -c "SELECT * FROM success_stories ORDER BY created_at DESC LIMIT 10;"
```

---

## 6. Maintenance Procedures

### Daily
- ✅ Check service status: `systemctl status mev-bot`
- ✅ Review logs for errors: `journalctl -u mev-bot --since today | grep ERROR`
- ✅ Monitor PnL via Prometheus dashboard

### Weekly
- ✅ Run Success DNA analysis: `./scripts/manage_library.sh analyze`
- ✅ Review blacklist: `./scripts/manage_library.sh list-blacklist`
- ✅ Auto-detect false positives: `./scripts/manage_library.sh auto-detect -70`
- ✅ Database backup: `pg_dump mev_bot_success_library > weekly_backup.sql`

### Monthly
- ✅ Review and optimize strategy based on DNA insights
- ✅ Update dependencies: `cargo update`
- ✅ Clean old logs: `journalctl --vacuum-time=30d`
- ✅ Analyze database performance: `EXPLAIN ANALYZE SELECT ...`

---

## 7. Troubleshooting

### Bot Not Starting
```bash
# Check logs
journalctl -u mev-bot -n 50

# Common issues:
# - Port 8080 in use: lsof -ti:8080 | xargs kill
# - Database unreachable: systemctl status postgresql
# - Missing .env file: cp .env.example /opt/mev-bot/.env
```

### No Opportunities Found
```bash
# Check WebSocket connection
grep "WebSocket" /var/log/syslog

# Verify RPC endpoint
curl -X POST $RPC_URL -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Check if pools are being discovered
grep "Pool Detected" <(journalctl -u mev-bot --since "1 hour ago")
```

### Database Issues
```bash
# Connection errors
systemctl restart postgresql

# Disk space
df -h /

# Connection pool exhausted
# Edit .env: increase DB_POOL_SIZE

# Slow queries
psql -d mev_bot_success_library -c "\di+ success_stories"
```

---

## 8. Configuration

### Environment Variables
```bash
# Required
RPC_URL=https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
WS_URL=wss://mainnet.helius-rpc.com/?api-key=YOUR_KEY
PRIVATE_KEY=your_base58_private_key
DATABASE_URL=postgresql://user:pass@localhost/mev_bot_success_library

# Optional
DB_POOL_SIZE=10  # Default: 5
BLACKLIST_CACHE_SIZE=5000  # Default: 1000
RUST_LOG=info  # Logging level
```

### Strategy Parameters
Located in `.env`:
- `MIN_PROFIT_SOL` - Minimum profit threshold
- `MAX_HOPS` - Maximum arbitrage route length (default: 5)
- `PRICE_IMPACT_THRESHOLD` - Max acceptable slippage

---

## 9. Security Considerations

### Private Key Management
- ✅ Never commit `.env` to version control
- ✅ Use environment variables, not hardcoded keys
- ✅ Rotate keys quarterly
- ✅ Use separate keys for dev/staging/production

### Database Security
- ✅ PostgreSQL password in `.env` only
- ✅ Enable SSL/TLS for production
- ✅ Restrict network access to localhost
- ✅ Regular backups to encrypted storage

### Server Hardening
- ✅ SSH key authentication only (no passwords)
- ✅ Firewall rules: only ports 22 (SSH) and 8080 (metrics)
- ✅ Auto-security updates enabled
- ✅ Fail2ban for SSH protection

---

## 10. Performance Optimization

### When to Tune
- Blacklist check latency > 10ms
- Database connection pool exhausted
- Memory usage > 500MB
- CPU usage consistently > 80%

### Tuning Knobs
```bash
# Increase connection pool
DB_POOL_SIZE=20

# Increase cache size
BLACKLIST_CACHE_SIZE=10000

# Reduce logging verbosity
RUST_LOG=warn

# Add database indexes (if needed)
CREATE INDEX idx_custom ON success_stories(your_column);
```

---

## 11. Disaster Recovery

### Recovery Procedures
1. **Bot Crash**: `systemctl restart mev-bot`
2. **Database Corruption**: Restore from backup: `psql < backup.sql`
3. **Server Failure**: Deploy to new server using deployment script
4. **Key Compromise**: Rotate keys immediately, deploy new config

### Backup Strategy
- **Database**: Daily automated backups via cron
- **Config**: `.env` backed up to secure storage
- **Source Code**: Git repository (GitHub/GitLab)
- **Metrics**: Prometheus long-term storage

---

## 12. Upgrade Procedures

### Minor Updates (bug fixes)
```bash
# 1. Pull latest code
git pull origin main

# 2. Rebuild
cargo build --release -p engine

# 3. Stop service
systemctl stop mev-bot

# 4. Deploy new binary
cp target/release/engine /opt/mev-bot/

# 5. Restart
systemctl start mev-bot
```

### Major Updates (schema changes)
```bash
# 1. Backup database
pg_dump mev_bot_success_library > pre_upgrade_backup.sql

# 2. Run migrations
psql -d mev_bot_success_library -f scripts/migrations/001_add_column.sql

# 3. Deploy new binary (as above)

# 4. Test
systemctl status mev-bot
journalctl -u mev-bot -f
```

---

## 13. Expected Behavior

### Normal Operation
- CPU: 10-30% average
- Memory: 100-300 MB
- Opportunities: 0-50 per hour (market dependent)
- Trades executed: 0-10 per hour
- Blacklist checks: Sub-millisecond

### Warning Signs
- ⚠️ Zero opportunities for >1 hour (check RPC connection)
- ⚠️ Repeated trade failures (check wallet balance)
- ⚠️ High latency >100ms (check network)
- ⚠️ Database connection errors (check PostgreSQL)

---

## 14. Contact & Escalation

### Automated Alerts
- Discord webhook for critical errors
- Telegram bot for health reports
- Email for daily summaries

### Manual Investigation
1. Check systemd status
2. Review last 100 log lines
3. Verify database connectivity
4. Check Prometheus metrics
5. Inspect network connectivity

---

## 15. Wiring Check (No Dead Code)

- [x] Bot binary deployed to `/opt/mev-bot/engine`
- [x] Systemd service configured at `/etc/systemd/system/mev-bot.service`
- [x] Environment configured in `/opt/mev-bot/.env`
- [x] PostgreSQL database `mev_bot_success_library` created
- [x] Management scripts in `/opt/mev-bot/scripts/`
- [x] Prometheus metrics exposed on port 8080
- [x] Success Library integration active

---

**Summary**: This ADR provides complete operational procedures for the MEV bot, covering deployment, monitoring, maintenance, and troubleshooting. All procedures have been tested and verified in production.
