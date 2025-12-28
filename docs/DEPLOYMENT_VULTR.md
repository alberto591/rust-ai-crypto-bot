# 🥇 Vultr New Jersey Deployment Guide

This guide details how to move your HFT bot from local monitoring to a permanent, low-latency home in Vultr New Jersey.

## 1. Instance Configuration
Deploy a new instance with these specific settings for maximum performance:
- **Location**: New Jersey (NJ)
- **Server Type**: Cloud Compute - Regular Performance
- **Plan**: $6/month (1GB RAM, 1 vCPU)
- **OS**: Ubuntu 24.04 LTS

## 2. Server Ignition
Once your instance is live, SSH into it:
```bash
ssh root@149.28.35.68
```

## 3. Automated Setup
Run our one-command installer to prepare the environment (Rust, dependencies, and build tools):
```bash
# Clone the repository (Assuming it's public or you have a token)
git clone https://github.com/lycanbeats/solana-mev-bot.git
cd solana-mev-bot
chmod +x scripts/setup_vultr.sh
./scripts/setup_vultr.sh
```

## 4. Wallet & Keys
You must securely provide your burner wallet keypair:
```bash
nano /root/solana-mev-bot/keypair.json
```
*Paste your [1,2,3...] byte array and save (Ctrl+O, Enter, Ctrl+X).*

## 5. Launching the V2 Engine
We use `tmux` to ensure the bot keeps running even if you close your terminal window.

### 📊 Pre-flight Production Checklist
Before launching, run this command to verify your "Ghost to Gold" parameters:
```bash
grep -E "EXECUTION_MODE|DEFAULT_TRADE_SIZE|MIN_PROFIT|AI_CONFIDENCE" .env
```
**Expected Production Values:**
- `EXECUTION_MODE=LiveMicro`
- `DEFAULT_TRADE_SIZE_LAMPORTS=20000000` (0.02 SOL)
- `MIN_PROFIT_THRESHOLD=100000`
- `AI_CONFIDENCE_THRESHOLD=0.85`

---

```bash
# Start a new persistent session
tmux new -s solana_bot

# Navigate and Launch
cd /root/solana-mev-bot
cargo run --release --bin engine -- --no-tui 2>&1 | tee bot.log

# DETACH from tmux: Press Ctrl+B, then D
```

## 6. Remote Monitoring
Since you are using **V2 Alerting**, you don't need to SSH in to check performance:
- Check **Telegram** for hourly reports.
- Use `/status` in Telegram for real-time snapshots.
- Use `/pause` if you need to stop trading from your phone.

---
> [!IMPORTANT]
> **Latency Advantage**: Running in New Jersey puts you physically closer to the Helius Virginia cluster, reducing network jitter and increasing your chance of landing bundles before other competitors.
