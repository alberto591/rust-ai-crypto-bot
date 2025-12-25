#!/bin/bash
# 10-Hour LiveMicro Production Test
# This script runs the MEV bot in LiveMicro mode (real trades, 0.02 SOL cap) for 10 hours

set -e

DURATION_HOURS=10
DURATION_SECONDS=$((DURATION_HOURS * 3600))
LOG_DIR="production_test_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$LOG_DIR"

echo "🚀 Starting 10-Hour LiveMicro Production Test"
echo "⚠️  WARNING: This will execute REAL TRADES on mainnet"
echo "💰 Wallet Balance: $(solana balance)"
echo "📁 Logs will be saved to: $LOG_DIR/"
echo ""
echo "Configuration:"
echo "  - Mode: LiveMicro (0.02 SOL cap per trade)"
echo "  - Duration: $DURATION_HOURS hours"
echo "  - Dynamic Jito Tips: 50% of profit"
echo "  - Volatility-Aware Slippage: Enabled"
echo ""
read -p "Press ENTER to start or Ctrl+C to cancel..."

# Build release binary
echo "🔨 Building release binary..."
cargo build -p engine --release

# Start the engine
echo "🔥 Starting engine..."
./target/release/engine > "$LOG_DIR/engine.log" 2>&1 &
ENGINE_PID=$!

echo "✅ Engine started (PID: $ENGINE_PID)"
echo "⏱️  Test will run for $DURATION_HOURS hours..."
echo ""

# Monitor and create snapshots
SNAPSHOT_INTERVAL=1800  # 30 minutes
SNAPSHOTS=$((DURATION_SECONDS / SNAPSHOT_INTERVAL))

for ((i=1; i<=SNAPSHOTS; i++)); do
    sleep $SNAPSHOT_INTERVAL
    
    ELAPSED=$((i * SNAPSHOT_INTERVAL))
    HOURS=$((ELAPSED / 3600))
    MINS=$(((ELAPSED % 3600) / 60))
    
    echo "📊 Snapshot $i/$SNAPSHOTS (${HOURS}h ${MINS}m elapsed)"
    
    # Capture current logs
    tail -n 100 "$LOG_DIR/engine.log" > "$LOG_DIR/snapshot_${i}.txt"
    
    # Get wallet balance
    echo "Balance: $(solana balance)" >> "$LOG_DIR/snapshot_${i}.txt"
    
    # Check if process is still running
    if ! kill -0 $ENGINE_PID 2>/dev/null; then
        echo "❌ Engine process died! Check logs at $LOG_DIR/engine.log"
        exit 1
    fi
done

echo "⏰ 10 hours completed!"
echo "🛑 Stopping engine..."
kill $ENGINE_PID
wait $ENGINE_PID 2>/dev/null || true

echo "✅ Test complete!"
echo ""
echo "📊 Generating summary..."

# Summary
echo "=== 10-Hour LiveMicro Test Summary ===" | tee "$LOG_DIR/summary.txt"
echo "" | tee -a "$LOG_DIR/summary.txt"
echo "Start Balance: Check first snapshot" | tee -a "$LOG_DIR/summary.txt"
echo "Final Balance: $(solana balance)" | tee -a "$LOG_DIR/summary.txt"
echo "" | tee -a "$LOG_DIR/summary.txt"

# Count opportunities
ARB_COUNT=$(grep -c "ARB_FOUND" "$LOG_DIR/engine.log" || echo "0")
echo "Opportunities Found: $ARB_COUNT" | tee -a "$LOG_DIR/summary.txt"

# Count profitable paths
PROFIT_COUNT=$(grep -c "Profitable path found" "$LOG_DIR/engine.log" || echo "0")
echo "Profitable Paths: $PROFIT_COUNT" | tee -a "$LOG_DIR/summary.txt"

# Count bundle dispatches
BUNDLE_COUNT=$(grep -c "BUNDLE DISPATCHED" "$LOG_DIR/engine.log" || echo "0")
echo "Bundles Sent: $BUNDLE_COUNT" | tee -a "$LOG_DIR/summary.txt"

# Count volatility adjustments
VOL_COUNT=$(grep -c "Volatility Detected" "$LOG_DIR/engine.log" || echo "0")
echo "Volatility Adjustments: $VOL_COUNT" | tee -a "$LOG_DIR/summary.txt"

echo "" | tee -a "$LOG_DIR/summary.txt"
echo "📁 Full logs available at: $LOG_DIR/engine.log"

echo ""
echo "🎯 Next Steps:"
echo "  1. Review logs: tail -f $LOG_DIR/engine.log"
echo "  2. Check summary: cat $LOG_DIR/summary.txt"
echo "  3. Analyze snapshots in $LOG_DIR/"
