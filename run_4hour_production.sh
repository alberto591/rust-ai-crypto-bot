#!/bin/bash

# Configuration
DURATION_SECONDS=$((4 * 3600))
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="logs/production_4hour_$TIMESTAMP.log"
SUMMARY_FILE="logs/production_summary_$TIMESTAMP.md"

mkdir -p logs

echo "🎯 Starting 4-Hour PRODUCTION Run..."
echo "📊 Mode: $(grep "EXECUTION_MODE" .env | head -n 1)"
echo "📊 Results will be saved to: $LOG_FILE"

# 1. Build the project
echo "🛠️ Rebuilding engine in release mode..."
cargo build -p engine --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed. Aborting."
    exit 1
fi

# 2. Run the engine with a timeout
echo "🚀 Production Run IN PROGRESS. Monitoring for 4 hours..."
echo "💡 You can follow logs with: tail -f $LOG_FILE"
./target/release/engine > "$LOG_FILE" 2>&1 &
ENGINE_PID=$!

# Trap Ctrl+C to ensure cleanup
trap "kill -SIGINT $ENGINE_PID; echo '\nInterrupted. Engine shutdown sent.'; exit" INT

# 3. Wait and Monitoring
ELAPSED=0
while [ $ELAPSED -lt $DURATION_SECONDS ]; do
    sleep 300
    ELAPSED=$((ELAPSED + 300))
    PERCENT=$((ELAPSED * 100 / DURATION_SECONDS))
    
    # Check if process is still running
    if ! kill -0 $ENGINE_PID 2>/dev/null; then
        echo "❌ Engine process died unexpectedly at $((ELAPSED / 60)) minutes."
        break
    fi

    # Extract latest periodic report for immediate feedback
    LATEST_REPORT=$(grep "PERIODIC REPORT" "$LOG_FILE" | tail -n 1)
    if [ -n "$LATEST_REPORT" ]; then
        echo "⏱️ [$PERCENT%] Progress: $((ELAPSED / 60))/240 min | $LATEST_REPORT"
    else
        echo "⏱️ [$PERCENT%] Progress: $((ELAPSED / 60))/240 min | waiting for first report..."
    fi
done

# 4. Shutdown
echo "🛑 Production window complete. Shutting down engine..."
kill -SIGINT "$ENGINE_PID"
sleep 10

# 5. Generate Summary
echo "📝 Generating Summary..."
{
    echo "# 4-Hour Production Run Summary"
    echo "Date: $(date)"
    echo "Log File: $LOG_FILE"
    echo ""
    echo "## 📊 Performance Statistics"
    # Extract the last summary block
    if grep -q "BOT PERFORMANCE SUMMARY" "$LOG_FILE"; then
        sed -n '/BOT PERFORMANCE SUMMARY/,/╚/p' "$LOG_FILE" | tail -n 25
    else
        echo "No performance summary block found in logs."
    fi
    
    echo ""
    echo "## 💡 Top Arbitrage Hits"
    grep "💡 Profitable path found" "$LOG_FILE" | head -n 15
    
    echo ""
    echo "## 🛡️ Risk & Safety"
    echo "- Risk blocks: $(grep -c "🚫 Trade blocked" "$LOG_FILE")"
    echo "- RPC Errors: $(grep -c "💥 Processing error" "$LOG_FILE")"
    echo "- Total Opps: $(grep -c "ARB_FOUND" "$LOG_FILE")"
    
    echo ""
    echo "## 📈 Telemetry Check"
    echo "- Highest recorded profit: $(grep "profit:" "$LOG_FILE" | awk '{print $NF}' | sort -nr | head -n 1) lamports"
} > "$SUMMARY_FILE"

echo "✅ Production Run Complete. Summary saved to $SUMMARY_FILE"
