#!/bin/bash
# start_bridge_run.sh - Robust 12-hour execution script for MacBook M1

echo "🛡️ Starting HFT Bridge Run..."

# 1. Clean up potential old instances
pkill -f "target/release/engine" || true

# 2. Check if binary exists
if [ ! -f "./target/release/engine" ]; then
    echo "❌ Error: Release binary not found. Build it first with: cargo build --release"
    exit 1
fi

# 3. Create a unique log file
LOG_FILE="logs/bridge_run_$(date +%Y%m%d_%H%M%S).log"
mkdir -p logs

echo "📡 Logging to: $LOG_FILE"
echo "☕ Using caffeinate to prevent sleep..."

# 4. Infinite Restart Loop
# This ensures that if the bot crashes or the connection drops, it restarts automatically.
echo "🚀 Bot is now running with AUTO-RESTART protection."
echo "📈 Use 'tail -f $LOG_FILE' to monitor."
echo "🛑 To stop everything: 'pkill -f engine' AND then 'control-C' this script."

while true; do
    echo "⚡ [$(date)] Engine Ignition..." >> "$LOG_FILE"
    caffeinate -is ./target/release/engine --no-tui < /dev/null 2>&1 | tee -a "$LOG_FILE"
    
    echo "⚠️ [$(date)] Engine exited. Restarting in 5 seconds..." >> "$LOG_FILE"
    sleep 5
done &
