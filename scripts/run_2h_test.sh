#!/bin/bash
# 2-Hour LiveMicro Test Run
set -e

echo "🧪 Starting 2-Hour LiveMicro Test"
echo "=================================="

# Check .env is set to LiveMicro
if ! grep -q "EXECUTION_MODE=LiveMicro" .env; then
    echo "Setting EXECUTION_MODE=LiveMicro..."
    sed -i.bak 's/EXECUTION_MODE=.*/EXECUTION_MODE=LiveMicro/' .env
fi

# Create log directory
mkdir -p logs
LOG_FILE="logs/test_2h_$(date +%Y%m%d_%H%M%S).log"

echo "📝 Log file: $LOG_FILE"
echo "⏱️  Duration: 2 hours"
echo ""

# Build release
echo "🔨 Building..."
cargo build --release 2>&1 | tee -a "$LOG_FILE"

echo ""
echo "🚀 Launching bot..."
echo "Press Ctrl+C to stop early"
echo ""

# Run with 2-hour timeout
timeout 2h cargo run --release --bin engine 2>&1 | tee -a "$LOG_FILE"

echo ""
echo "✅ Test complete!"
echo "📊 Check metrics at: http://localhost:9090/metrics"
echo "📝 Full log: $LOG_FILE"
