#!/usr/bin/env bash
# Vultr Production Deployment Script
# Deploys PostgreSQL, Success Library, and MEV Bot to Vultr server

set -e

SSH_KEY="~/.ssh/vultr_mev_bot"
SERVER="root@149.28.35.68"
SSH_CMD="ssh -i $SSH_KEY $SERVER"

echo "🚀 MEV Bot - Vultr Production Deployment"
echo "=========================================="
echo ""

# Phase 1: Install PostgreSQL
echo "📦 Phase 1: Installing PostgreSQL..."
$SSH_CMD << 'ENDSSH'
apt-get update -qq
apt-get install -y postgresql postgresql-contrib
systemctl enable postgresql
systemctl start postgresql
echo "✅ PostgreSQL installed"
ENDSSH

# Phase 2: Create Database and User
echo "📊 Phase 2: Setting up database..."
$SSH_CMD << 'ENDSSH'
sudo -u postgres psql << 'EOSQL'
CREATE DATABASE mev_bot_success_library;
CREATE USER mevbot WITH ENCRYPTED PASSWORD 'CHANGE_ME_IN_PRODUCTION';
GRANT ALL PRIVILEGES ON DATABASE mev_bot_success_library TO mevbot;
\c mev_bot_success_library
GRANT ALL ON SCHEMA public TO mevbot;
EOSQL
echo "✅ Database created"
ENDSSH

# Phase 3: Deploy Schema
echo "📋 Phase 3: Deploying database schema..."
scp -i $SSH_KEY scripts/init_db.sql $SERVER:/tmp/init_db.sql
$SSH_CMD "sudo -u postgres psql -d mev_bot_success_library -f /tmp/init_db.sql"
echo "✅ Schema deployed"

# Phase 4: Build and Deploy Bot
echo "🔨 Phase 4: Building bot binary..."
cargo build --release -p engine
echo "✅ Binary built"

echo "📤 Phase 4.1: Uploading bot..."
scp -i $SSH_KEY target/release/engine $SERVER:/opt/mev-bot/engine
scp -i $SSH_KEY .env $SERVER:/opt/mev-bot/.env
echo "✅ Bot uploaded"

# Phase 5: Create systemd service
echo "⚙️  Phase 5: Configuring systemd service..."
cat > /tmp/mev-bot.service << 'EOF'
[Unit]
Description=MEV Arbitrage Bot
After=network.target postgresql.service

[Service]
Type=simple
User=root
WorkingDirectory=/opt/mev-bot
ExecStart=/opt/mev-bot/engine
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
EOF

scp -i $SSH_KEY /tmp/mev-bot.service $SERVER:/etc/systemd/system/
$SSH_CMD "systemctl daemon-reload && systemctl enable mev-bot"
echo "✅ Systemd service configured"

# Phase 6: Update .env with production DATABASE_URL
echo "🔧 Phase 6: Configuring environment..."
$SSH_CMD "sed -i 's|^DATABASE_URL=.*|DATABASE_URL=postgresql://mevbot:CHANGE_ME_IN_PRODUCTION@localhost/mev_bot_success_library|' /opt/mev-bot/.env"
echo "✅ Environment configured"

# Phase 7: Start the bot
echo "🎯 Phase 7: Starting bot service..."
$SSH_CMD "systemctl start mev-bot"
sleep 3
$SSH_CMD "systemctl status mev-bot --no-pager"
echo "✅ Bot started"

echo ""
echo "🎉 Deployment Complete!"
echo ""
echo "📊 Next steps:"
echo "  - Monitor logs: ssh $SERVER 'journalctl -u mev-bot -f'"
echo "  - Check status: ssh $SERVER 'systemctl status mev-bot'"
echo "  - View metrics: curl http://149.28.35.68:8080/metrics"
echo "  - Manage library: ssh $SERVER 'cd /opt/mev-bot && ./scripts/manage_library.sh stats'"
