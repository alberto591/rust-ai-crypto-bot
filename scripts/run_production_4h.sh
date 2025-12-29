#!/bin/bash
# 4-Hour Production Stability Test
# Runs bot with automated logging every 5 minutes for stability verification.

# Ensure environment is clean
./scripts/cleanup_engine.sh

LOG_DIR="prod_run_4h_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$LOG_DIR"

echo "🚀 Starting 4-hour production stability test..."
echo "📁 Logs will be saved to: $LOG_DIR"

# Start the bot in background (Release mode for performance)
# Using --release is critical for HFT/MEV to handle high log volume
cargo run --package engine --release > "$LOG_DIR/bot_output.log" 2>&1 &
BOT_PID=$!

echo "✅ Bot started (PID: $BOT_PID)"
echo "⏱️  Test duration: 4 hours"
echo "📊 Snapshot interval: 5 minutes"
echo ""

# Create snapshot function
snapshot() {
    local snapshot_num=$1
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    
    echo "[$timestamp] Snapshot $snapshot_num" >> "$LOG_DIR/snapshots.log"
    
    {
        echo "=== Snapshot $snapshot_num at $timestamp ==="
        echo "Uptime: $(ps -p $BOT_PID -o etime= || echo 'DEAD')"
        echo ""
        echo "--- WebSocket Status ---"
        grep "Watcher WebSocket" "$LOG_DIR/bot_output.log" | tail -n 3 || echo "Stable"
        echo ""
        echo "--- Pump.fun Hydration ---"
        grep "Hydrated Pump.fun Curve" "$LOG_DIR/bot_output.log" | tail -n 5 || echo "No hydrations yet"
        echo ""
        echo "--- Opportunities ---"
        grep "OPPORTUNITY" "$LOG_DIR/bot_output.log" | tail -n 5 || echo "None detected"
        echo ""
        echo "--- Recent Successes ---"
        grep "✅" "$LOG_DIR/bot_output.log" | tail -n 5 || echo "None"
        echo ""
        echo "--- Error Count ---"
        grep -i "error" "$LOG_DIR/bot_output.log" | wc -l
        echo "=========================================="
        echo ""
    } > "$LOG_DIR/snapshot_$snapshot_num.txt"
    
    echo "  ✓ Snapshot $snapshot_num saved"
}

# Run for 4 hours with 5-minute intervals
DURATION_HOURS=4
INTERVAL_MINUTES=5
TOTAL_SNAPSHOTS=$((DURATION_HOURS * 60 / INTERVAL_MINUTES))

for i in $(seq 1 $TOTAL_SNAPSHOTS); do
    elapsed=$((i * INTERVAL_MINUTES))
    remaining=$((DURATION_HOURS * 60 - elapsed))
    
    # Check if bot is still alive
    if ! kill -0 $BOT_PID 2>/dev/null; then
        echo "❌ CRITICAL: Bot process (PID: $BOT_PID) has died!"
        echo "Check $LOG_DIR/bot_output.log for clues."
        break
    fi

    echo "⏰ [$elapsed min elapsed, $remaining min remaining]"
    snapshot $i
    
    if [ $i -lt $TOTAL_SNAPSHOTS ]; then
        sleep $((INTERVAL_MINUTES * 60))
    fi
done

# Final snapshot and summary
echo ""
echo "🏁 4-Hour Test complete! Generating report..."
snapshot "FINAL"

# Stop the bot gracefully
kill -INT $BOT_PID 2>/dev/null
sleep 2

# Generate summary
{
    echo "# 4-Hour Production Stability Report"
    echo "Generated at: $(date)"
    echo ""
    echo "## Execution Summary"
    echo "- Total runtime: 4 hours"
    echo "- Log Directory: $LOG_DIR"
    echo ""
    
    echo "### Hydration Metrics"
    total_hydrated=$(grep -c "Hydrated Pump.fun Curve" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- Total Pump.fun Pools Hydrated: $total_hydrated"
    
    echo "### WebSocket Stability"
    ws_fails=$(grep -c "Watcher WebSocket Failed" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- WebSocket Failures/Retries: $ws_fails"
    
    echo "### Errors"
    error_count=$(grep -ic "error" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- Total Errors: $error_count"
    
    echo ""
    echo "Full logs available in $LOG_DIR/"
} > "$LOG_DIR/PROD_REPORT.md"

echo "✅ Report generated: $LOG_DIR/PROD_REPORT.md"
