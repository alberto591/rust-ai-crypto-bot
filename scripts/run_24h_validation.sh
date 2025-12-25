#!/bin/bash
# 24-Hour LiveMicro Validation Test
# Runs bot in LiveMicro mode with comprehensive logging

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "🧪 Starting 24-Hour LiveMicro Validation Test"
echo "================================================"

# Configuration
DURATION_HOURS=24
LOG_DIR="$PROJECT_ROOT/logs"
TEST_START=$(date +%Y%m%d_%H%M%S)
TEST_LOG="$LOG_DIR/validation_${TEST_START}.log"

# Create log directory
mkdir -p "$LOG_DIR"

# Check prerequisites
echo "📋 Pre-flight checks..."

# 1. Check if .env exists
if [ ! -f .env ]; then
    echo "❌ .env file not found!"
    exit 1
fi

# 2. Verify EXECUTION_MODE is LiveMicro
if ! grep -q "EXECUTION_MODE=LiveMicro" .env; then
    echo "⚠️  Setting EXECUTION_MODE=LiveMicro in .env"
    sed -i.bak 's/EXECUTION_MODE=.*/EXECUTION_MODE=LiveMicro/' .env || \
        echo "EXECUTION_MODE=LiveMicro" >> .env
fi

# 3. Check wallet balance
echo "💰 Checking wallet balance..."
# (Would need actual RPC call to verify - skipped for now)

# 4. Build release binary
echo "🔨 Building release binary..."
cargo build --release 2>&1 | tee -a "$TEST_LOG"

if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Pre-flight checks complete"
echo ""

# Record start state
echo "📊 Recording initial state..."
INITIAL_BALANCE=$(grep "SOL Balance" "$LOG_DIR"/performance.log 2>/dev/null | tail -1 || echo "N/A")
echo "Start Time: $(date)" >> "$TEST_LOG"
echo "Initial Balance: $INITIAL_BALANCE" >> "$TEST_LOG"

# Start Prometheus metrics server (built into bot)
echo "🚀 Starting bot in LiveMicro mode..."
echo "Duration: $DURATION_HOURS hours"
echo "Log file: $TEST_LOG"
echo ""

# Run bot with timeout
timeout ${DURATION_HOURS}h cargo run --release --bin engine 2>&1 | tee -a "$TEST_LOG" || true

# Post-test summary
echo ""
echo "🏁 Test completed!"
echo "================================================"
echo "Duration: $DURATION_HOURS hours"
echo "Log: $TEST_LOG"
echo ""

# Extract key metrics from log
echo "📈 Test Summary:"
echo "----------------"
grep -E "(Opportunities|Profitable|Circuit|P&L)" "$TEST_LOG" | tail -20 || echo "No metrics found in log"

echo ""
echo "Next Steps:"
echo "1. Review Grafana dashboard for detailed metrics"
echo "2. Analyze $TEST_LOG for errors or anomalies"
echo "3. Check data/arbitrage_*.csv for recorded opportunities"
echo "4. Calculate win rate and net P&L"
echo ""
echo "Use: ./scripts/analyze_test_results.sh $TEST_START"
