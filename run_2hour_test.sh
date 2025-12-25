#!/bin/bash
# 2-Hour Production Readiness Test
# Runs bot with automated logging every 5 minutes

LOG_DIR="test_run_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$LOG_DIR"

echo "🚀 Starting 2-hour production readiness test..."
echo "📁 Logs will be saved to: $LOG_DIR"

# Start the bot in background
cargo run --package engine --release > "$LOG_DIR/bot_output.log" 2>&1 &
BOT_PID=$!

echo "✅ Bot started (PID: $BOT_PID)"
echo "⏱️  Test duration: 2 hours"
echo "📊 Logging interval: 5 minutes"
echo ""

# Create snapshot function
snapshot() {
    local snapshot_num=$1
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    
    echo "[$timestamp] Snapshot $snapshot_num" >> "$LOG_DIR/snapshots.log"
    
    # Capture key metrics
    {
        echo "=== Snapshot $snapshot_num at $timestamp ==="
        echo ""
        echo "--- ARB Opportunities Found ---"
        grep "ARB_FOUND" "$LOG_DIR/bot_output.log" | tail -n 10 || echo "None found yet"
        echo ""
        echo "--- Cross-DEX Pool Additions ---"
        grep "Added new pool" "$LOG_DIR/bot_output.log" | tail -n 5 || echo "None"
        echo ""
        echo "--- Recent Cycles Detected ---"
        grep "CYCLE DETECTED" "$LOG_DIR/bot_output.log" | tail -n 5 || echo "None"
        echo ""
        echo "--- Error Count ---"
        grep -i "error" "$LOG_DIR/bot_output.log" | wc -l
        echo ""
        echo "--- Graph Stats ---"
        grep "Graph Updated" "$LOG_DIR/bot_output.log" | tail -n 1 || echo "No updates"
        echo ""
        echo "=========================================="
        echo ""
    } > "$LOG_DIR/snapshot_$snapshot_num.txt"
    
    echo "  ✓ Snapshot $snapshot_num saved"
}

# Run for 2 hours with 5-minute intervals
DURATION_HOURS=2
INTERVAL_MINUTES=5
TOTAL_SNAPSHOTS=$((DURATION_HOURS * 60 / INTERVAL_MINUTES))

for i in $(seq 1 $TOTAL_SNAPSHOTS); do
    elapsed=$((i * INTERVAL_MINUTES))
    remaining=$((DURATION_HOURS * 60 - elapsed))
    
    echo "⏰ [$elapsed min elapsed, $remaining min remaining]"
    snapshot $i
    
    if [ $i -lt $TOTAL_SNAPSHOTS ]; then
        sleep $((INTERVAL_MINUTES * 60))
    fi
done

# Final snapshot
echo ""
echo "🏁 Test complete! Creating final report..."
snapshot "FINAL"

# Kill the bot
kill $BOT_PID 2>/dev/null

# Generate summary
{
    echo "# 2-Hour Production Readiness Test Report"
    echo "Test completed at: $(date)"
    echo ""
    echo "## Summary Statistics"
    echo ""
    echo "- Total runtime: 2 hours"
    echo "- Snapshots captured: $TOTAL_SNAPSHOTS + 1 final"
    echo ""
    echo "### Arbitrage Opportunities"
    total_arbs=$(grep -c "ARB_FOUND" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- Total ARB_FOUND: $total_arbs"
    echo ""
    echo "### Cross-DEX Integration"
    total_pools=$(grep -c "Added new pool" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- Pools added to edges: $total_pools"
    echo ""
    echo "### Cycles Detected"
    total_cycles=$(grep -c "CYCLE DETECTED" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- Total cycles: $total_cycles"
    echo ""
    echo "### Errors"
    error_count=$(grep -ic "error" "$LOG_DIR/bot_output.log" || echo "0")
    echo "- Error count: $error_count"
    echo ""
    echo "See individual snapshots in $LOG_DIR/ for detailed analysis."
} > "$LOG_DIR/SUMMARY.md"

echo ""
echo "✅ Test complete!"
echo "📊 Summary: $LOG_DIR/SUMMARY.md"
echo "📁 All logs: $LOG_DIR/"
