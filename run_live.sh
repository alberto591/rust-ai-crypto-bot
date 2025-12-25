#!/bin/bash
# run_live.sh
# Safely launches the Solana MEV Bot with TUI enabled

echo "🚀 Launching Solana MEV Bot (LiveMicro Mode)..."
echo "⚠️  Ensure you are monitoring the TUI. Press 'q' or Ctrl+C to stop."
echo "---------------------------------------------------"

# Run with release optimizations for better latency, or debug for now
# Using -p engine (package engine)
mkdir -p logs
cargo run -p engine > logs/debug.log 2>&1 &
PID=$!
echo "Bot running in background (PID: $PID). TUI is hidden."
echo "Tailing logs/debug.log..."
tail -f logs/debug.log

echo "---------------------------------------------------"
echo "🛑 Bot stopped."
