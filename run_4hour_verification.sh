#!/bin/bash

# Configuration
DURATION_SECONDS=$((4 * 3600))
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="logs/verification_4hour_$TIMESTAMP.log"
SUMMARY_FILE="logs/verification_summary_$TIMESTAMP.md"

mkdir -p logs

echo "🎯 Starting 4-Hour Verification Simulation..."
echo "📊 Results will be saved to: $LOG_FILE"

# 1. Update .env for Simulation Mode
echo "📝 Configuring .env for Simulation..."
cp .env .env.bak
# Use a portable sed approach for macOS find/replace
sed -i '' 's/^EXECUTION_MODE=.*/EXECUTION_MODE=Simulation/' .env
sed -i '' 's/^FORCED_ARB_MOCK=.*/FORCED_ARB_MOCK=false/' .env

# 2. Build the project
echo "🛠️ Rebuilding engine..."
cargo build -p engine --release

# 3. Run the engine with a timeout
echo "🚀 Simulation IN PROGRESS. Monitoring for 4 hours..."
echo "💡 You can follow logs with: tail -f $LOG_FILE"
./target/release/engine > "$LOG_FILE" 2>&1 &
ENGINE_PID=$!

# Trap Ctrl+C to ensure cleanup if the user interrupts early
trap "kill -SIGINT $ENGINE_PID; mv .env.bak .env; echo '\nInterrupted. Restored .env'; exit" INT

# 4. Wait
# Instead of one big sleep, we sleep in 5-minute chunks to show progress
ELAPSED=0
while [ $ELAPSED -lt $DURATION_SECONDS ]; do
    sleep 300
    ELAPSED=$((ELAPSED + 300))
    PERCENT=$((ELAPSED * 100 / DURATION_SECONDS))
    
    # Extract latest periodic report for immediate feedback
    LATEST_REPORT=$(grep "PERIODIC REPORT" "$LOG_FILE" | tail -n 1)
    if [ -n "$LATEST_REPORT" ]; then
        echo "⏱️ [$PERCENT%] Progress: $((ELAPSED / 60))/240 min | $LATEST_REPORT"
    else
        echo "⏱️ [$PERCENT%] Progress: $((ELAPSED / 60))/240 min | waiting for first report..."
    fi
done

# 5. Shutdown
echo "🛑 Simulation window complete. Shutting down engine..."
kill -SIGINT "$ENGINE_PID"
sleep 5

# 6. Generate Summary
echo "📝 Generating Summary..."
{
    echo "# 4-Hour Verification Summary"
    echo "Date: $(date)"
    echo "Mode: Simulation"
    echo ""
    echo "## 📊 Performance Statistics"
    # Extract the last summary block
    if grep -q "BOT PERFORMANCE SUMMARY" "$LOG_FILE"; then
        sed -n '/BOT PERFORMANCE SUMMARY/,/╚/p' "$LOG_FILE" | tail -n 20
    else
        echo "No performance summary block found in logs."
    fi
    
    echo ""
    echo "## 💡 Top Arbitrage Hits"
    grep "💡 Profitable path found" "$LOG_FILE" | head -n 10
    
    echo ""
    echo "## 🛡️ Risk & Safety"
    echo "- Risk blocks: $(grep -c "🚫 Trade blocked" "$LOG_FILE")"
    echo "- RPC Errors: $(grep -c "💥 Processing error" "$LOG_FILE")"
    
    echo ""
    echo "## 📈 Telemetry Check"
    echo "- Highest recorded profit: $(grep "profit:" "$LOG_FILE" | awk '{print $NF}' | sort -nr | head -n 1) lamports"
} > "$SUMMARY_FILE"

# 7. Restore .env
mv .env.bak .env

echo "✅ Verification Complete. Summary saved to $SUMMARY_FILE"
