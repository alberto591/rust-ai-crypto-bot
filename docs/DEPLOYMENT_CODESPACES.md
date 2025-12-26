# GitHub Codespaces Deployment Guide (5-Day Validation) 🚀

This guide follows the optimized 7-step plan for deploying the Rust HFT Bot to GitHub Codespaces.

## 1. Preparation (Local) 📂
Ensure your local code is ready for the cloud:
```bash
# Create .gitignore if missing
cat > .gitignore <<EOF
/target
.env
*.log
Cargo.lock
EOF

# Commit latest changes
git add .
git commit -m "Initial commit for cloud testing"

# Push to your repo
git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPO.git
git push -u origin main
```

## 2. Launch Codespace 🌐
1. Navigate to your repository on GitHub.
2. Click **Code** -> **Codespaces** tab -> **Create codespace on main**.

## 3. Automated Setup 🛠️
Once the terminal opens in the Codespace, run the setup script:
```bash
curl -sSL https://raw.githubusercontent.com/alberto591/rust-ai-crypto-bot/main/scripts/setup_codespaces.sh | bash
```

## 4. Configuration 🔑
1. **Edit .env**: `nano .env` (Add your Helius/Jito keys).
2. **Add Keypair**: `nano keypair.json` (Paste your BURNER wallet JSON).

## 5. Build & Test 🏗️
```bash
# Build release version
cargo build --release

# 30-second connectivity test
timeout 30s cargo run --release --package engine -- --no-tui
```

## 6. Persistent Run (tmux) 🔄
```bash
# Start tmux session
tmux new -s solana_bot

# Start the bot with timestamped logging
cargo run --release --package engine -- --no-tui 2>&1 | tee bot_run_$(date +%Y%m%d_%H%M%S).log

# Detach: Press Ctrl+B, then D
```

## 💡 Keeping the Codespace Alive
Codespaces auto-suspend after inactivity. To prevent this:
1. **Settings**: Go to `github.com/settings/codespaces` and set "Default idle timeout" to 4 hours.
2. **Keepalive Pane**: In a new tmux pane (Ctrl+B, then %), run:
   ```bash
   while true; do echo "Keepalive: $(date)"; sleep 300; done
   ```

## 📊 Monitoring
- **Attach**: `tmux attach -t solana_bot`
- **Check Logs**: `tail -f bot_run_*.log`
- **Check Stats**: `grep "OPPORTUNITY" bot_run_*.log | wc -l`
