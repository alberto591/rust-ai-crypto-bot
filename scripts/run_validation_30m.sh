#!/bin/bash

# Configuration
DURATION_SECONDS=1800
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="logs/validation_30m_$TIMESTAMP.log"
SUMMARY_FILE="logs/validation_summary_$TIMESTAMP.md"

mkdir -p logs

echo "🎯 Starting 30-Minute STRATEGY VALIDATION Run..."
echo "📊 Results will be saved to: $LOG_FILE"

# 1. Build the project
echo "🛠️ Rebuilding engine in release mode..."
cargo build -p engine --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed. Aborting."
    exit 1
fi

# 2. Run the engine with a timeout
echo "🚀 Validation Run IN PROGRESS. Monitoring for 30 minutes..."
./target/release/engine > "$LOG_FILE" 2>&1 &
ENGINE_PID=$!

# Trap Ctrl+C to ensure cleanup
trap "kill -SIGINT $ENGINE_PID; echo '\nInterrupted. Engine shutdown sent.'; exit" INT

# 3. Wait and Monitoring
ELAPSED=0
while [ $ELAPSED -lt $DURATION_SECONDS ]; do
    sleep 60
    ELAPSED=$((ELAPSED + 60))
    PERCENT=$((ELAPSED * 100 / DURATION_SECONDS))
    
    # Check if process is still running
    if ! kill -0 $ENGINE_PID 2>/dev/null; then
        echo "❌ Engine process died unexpectedly at $((ELAPSED / 60)) minutes."
        break
    fi

    # Extract latest periodic report for immediate feedback
    LATEST_REPORT=$(grep "PERIODIC REPORT" "$LOG_FILE" | tail -n 1)
    if [ -n "$LATEST_REPORT" ]; then
        echo "⏱️ [$PERCENT%] Progress: $((ELAPSED / 60))/30 min | $LATEST_REPORT"
    else
        echo "⏱️ [$PERCENT%] Progress: $((ELAPSED / 60))/30 min | waiting for first report..."
    fi
done

# 4. Shutdown
echo "🛑 Validation window complete. Shutting down engine..."
kill -SIGINT "$ENGINE_PID"
sleep 5

# 5. Generate Summary
echo "📝 Generating Summary..."
{
    echo "# 30-Minute Strategy Validation Summary"
    echo "Date: $(date)"
    echo "Log File: $LOG_FILE"
    echo ""
    echo "## 🧬 Intelligence & DNA Matching"
    echo "- DNA Matches: $(grep -c "🧬 DNA Match!" "$LOG_FILE")"
    echo "- DNA Gate Rejections: $(grep -c "⛔ DNA GATE" "$LOG_FILE")"
    echo "- AI Rejections: $(grep -c "⚠️ Opportunity rejected by AI Model" "$LOG_FILE")"
    
    echo ""
    echo "## 🛡️ Safety & Latency"
    echo "- Safety Rejections: $(grep -c "⛔ SAFETY:" "$LOG_FILE")"
    echo "- Blacklist Hits: $(grep -c "⛔ Token .* FAILED safety validation" "$LOG_FILE")"
    echo "- Avg process_event duration: $(grep "Duration:" "$LOG_FILE" | awk '{sum+=$5; count++} END {if (count > 0) print sum/count; else print 0}') ms"

    echo ""
    echo "## 📊 Execution"
    echo "- Bundle Dispatches: $(grep -c "🔥 BUNDLE DISPATCHED" "$LOG_FILE")"
    echo "- Total Opps Found: $(grep -c "ARB_FOUND" "$LOG_FILE")"
    echo "- Simulation Fails: $(grep -c "❌ Simulation fail" "$LOG_FILE")"
} > "$SUMMARY_FILE"

echo "✅ Validation Run Complete. Summary saved to $SUMMARY_FILE"
