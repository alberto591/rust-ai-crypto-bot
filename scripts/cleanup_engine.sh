#!/bin/bash
# scripts/cleanup_engine.sh
# Purges any ghost engine processes to ensure a clean ignition.

echo "🧹 Cleaning up ghost engine processes..."

# Kill any 'engine' or 'cargo run' processes related to the bot
pkill -f "target/release/engine"
pkill -f "cargo run --package engine"

# Optional: Clean up any stale lock files if they exist
# rm -f /tmp/bot.lock

echo "✅ Environment cleaned. Ready for ignition."
