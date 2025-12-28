#!/usr/bin/env bash
# Knowledge Transfer Script
# Exports local Success Library data and imports it into the Vultr Production DB

set -e

LOCAL_DB="mev_bot_success_library"
VULTR_IP="149.28.35.68"
VULTR_USER="root"
SSH_KEY="~/.ssh/vultr_mev_bot"

echo "🧠 Starting Knowledge Transfer: Local -> Vultr"
echo "============================================"

# 1. Export local data
echo "📦 Exporting local success stories..."
/opt/homebrew/opt/postgresql@17/bin/pg_dump -a -t success_stories --inserts -d $LOCAL_DB > /tmp/success_stories_transfer.sql

# 2. Upload to Vultr
echo "🚀 Uploading to Vultr..."
scp -i $SSH_KEY /tmp/success_stories_transfer.sql $VULTR_USER@$VULTR_IP:/tmp/success_stories_transfer.sql

# 3. Import into Vultr DB
echo "📥 Importing into production database..."
ssh -i $SSH_KEY $VULTR_USER@$VULTR_IP "sudo -u postgres psql -d mev_bot_success_library -f /tmp/success_stories_transfer.sql"

echo "✅ Knowledge Transfer Complete!"
ssh -i $SSH_KEY $VULTR_USER@$VULTR_IP "sudo -u postgres psql -d mev_bot_success_library -c 'SELECT COUNT(*) as production_stories FROM success_stories;'"
