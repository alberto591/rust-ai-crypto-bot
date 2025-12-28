#!/usr/bin/env bash
# Production Activation Script
# Moves the compiled binary to production and starts the systemd service

set -e

echo "🚀 Activating Production Mojito Bot on Vultr..."
echo "============================================="

# 1. Stop existing service if running
echo "🛑 Stopping mev-bot service..."
systemctl stop mev-bot || true

# 2. Deploy binary
echo "📦 Deploying binary to /opt/mev-bot/..."
cp /opt/mev-bot-src/target/release/engine /opt/mev-bot/engine
chmod +x /opt/mev-bot/engine

# 3. Reload and Start service
echo "🔄 Reloading and starting systemd service..."
systemctl daemon-reload
systemctl enable mev-bot
systemctl start mev-bot

echo "✅ Production Bot Activated!"
systemctl status mev-bot
