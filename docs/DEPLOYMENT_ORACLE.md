# Oracle Cloud Deployment Guide (Free Tier) ☁️

This guide covers deploying the Rust HFT Bot to an Oracle Cloud ARM-based instance for persistent 24/7 testing.

## 1. Instance Provisioning 🛠️
Log into your [Oracle Cloud Console](https://cloud.oracle.com/) and follow these steps:

### Specifications (A1.Flex - Always Free)
- **Image**: Oracle Linux 8 (or Ubuntu 22.04)
- **Shape**: `VM.Standard.A1.Flex`
- **OCPUs**: 4
- **Memory**: 24 GB
- **Compartment**: (Your Default)

### Networking (VCN)
Ensure your Security List allows:
- **TCP Port 22**: SSH (Default)
- **TCP Port 8080**: Prometheus Metrics (Optional/Restricted)

## 2. One-Click Setup 🚀
Once connected via SSH, run the following to prepare the environment:

```bash
# Download and run the deployment script
curl -o setup.sh https://raw.githubusercontent.com/alberto591/rust-ai-crypto-bot/main/scripts/deploy_oracle.sh
chmod +x setup.sh
./setup.sh
```

## 3. Running with tmux 🔄
The bot should be run inside a `tmux` session to ensure it continues running after you disconnect.

### Start a new session:
```bash
tmux new -s bot
```

### Run the bot:
```bash
cd rust-ai-crypto-bot
cargo run --release --package engine -- --no-tui
```

### Detach and Re-attach:
- **To Detach**: Press `Ctrl+B` then `D`.
- **To Re-attach**: Run `tmux attach -t bot`.

## 4. Monitoring 📊
- Check your **Discord/Telegram** every 30 minutes for automated status reports.
- If you enabled port 8080, check metrics at `http://<YOUR_ORACLE_IP>:8080/metrics`.

---
> [!TIP]
> Use `crontab -e` with `@reboot tmux new-session -d -s bot 'cd ~/rust-ai-crypto-bot && cargo run --release --package engine -- --no-tui'` to ensure the bot restarts if the instance reboots.
