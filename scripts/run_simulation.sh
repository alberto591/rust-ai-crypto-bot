#!/bin/bash
# scripts/run_simulation.sh
# Runs the Solana MEV bot for a fixed duration and archives results.

DURATION=600 # 10 minutes
LOG_DIR="logs"
DATA_DIR="data"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
ARCHIVE_NAME="simulation_run_${TIMESTAMP}.tar.gz"

echo "🚀 Starting 10-minute simulation..."
echo "⏱️  Duration: $DURATION seconds"

# Ensure directories exist
mkdir -p "$LOG_DIR"
mkdir -p "$DATA_DIR"

# Start the bot in the background
# We use nohup to ensure it doesn't die if the terminal closes, 
# and redirect stdout/stderr to a debug log.
# However, for the user to see the TUI, we should ideally run it in a way they can monitor.
# But since this is an automated task, we'll run it in background and capture output.
cargo run --release --bin engine > "$LOG_DIR/simulation_debug.log" 2>&1 &
BOT_PID=$!

echo "🤖 Bot started with PID: $BOT_PID"
echo "📊 Monitoring progress for 10 minutes..."

# Wait for the specified duration
sleep "$DURATION"

echo "🛑 Time's up! Shutting down the bot..."

# Graceful shutdown (SIGINT like Ctrl+C)
kill -INT "$BOT_PID"

# Wait a bit for it to cleanup
sleep 5

# Check if still running, if so, force kill
if ps -p $BOT_PID > /dev/null; then
   echo "⚠️ Bot did not exit gracefully, forcing shutdown..."
   kill -9 "$BOT_PID"
fi

echo "📦 Archiving logs and data..."
tar -czf "$ARCHIVE_NAME" "$LOG_DIR" "$DATA_DIR"

echo "✅ Simulation complete. Logs saved to $ARCHIVE_NAME"
echo "🔍 You can analyze $LOG_DIR/performance.log and $DATA_DIR content."
