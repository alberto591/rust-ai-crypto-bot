#!/bin/bash
# 🏁 Production Launch Script
# Target: Ubuntu 24.04 (Vultr NJ)

echo "🚀 Preparing Formula 1 Production Launch..."

# 1. System Setups (Security)
echo "🛡️ Installing security hardening (fail2ban)..."
sudo apt update && sudo apt install -y fail2ban
sudo systemctl enable fail2ban
sudo systemctl start fail2ban

# 2. Service Installation
echo "⚙️ Installing systemd service..."
# Ensure the scripts directory exists in the target
mkdir -p /root/solana-mev-bot/scripts
sudo cp /root/solana-mev-bot/scripts/solana-bot.service /etc/systemd/system/solana-bot.service
sudo systemctl daemon-reload
sudo systemctl enable solana-bot

# 3. Log Rotation
echo "📋 Configuring log rotation (prevent disk fill)..."
printf "/root/solana-mev-bot/logs/*.log {\n    daily\n    rotate 7\n    compress\n    delaycompress\n    missingok\n    notifempty\n    create 0640 root root\n}\n" | sudo tee /etc/logrotate.d/solana-bot > /dev/null

# 4. Starting the Engine
echo "🦾 Igniting Engine..."
sudo systemctl start solana-bot

echo ""
echo "✅ Production Launch Complete!"
echo "--------------------------------------------------"
echo "📍 Monitor logs: journalctl -u solana-bot -f"
echo "📍 View live data: tail -f /root/solana-mev-bot/logs/engine.log"
echo "📍 Check status: systemctl status solana-bot"
echo "--------------------------------------------------"
