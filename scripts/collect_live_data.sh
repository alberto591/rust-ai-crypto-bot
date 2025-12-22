#!/bin/bash
# Live Data Collection Script for Solana MEV Bot
# This script runs the bot in DRY_RUN mode to collect real market data for AI training

set -e

echo "🚀 Starting Solana MEV Bot - Live Data Collection Mode"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Configuration
export SIMULATION=false
export DRY_RUN=true
DURATION_HOURS=${1:-24}
DATA_DIR="data"

# Calculate duration in seconds
DURATION_SECONDS=$((DURATION_HOURS * 3600))

echo "📊 Configuration:"
echo "  - Mode: DRY_RUN (no real transactions)"
echo "  - Duration: ${DURATION_HOURS} hours"
echo "  - Data Directory: ${DATA_DIR}"
echo "  - Simulation: OFF (live market data)"
echo ""

# Check if data directory exists
if [ ! -d "$DATA_DIR" ]; then
    mkdir -p "$DATA_DIR"
    echo "✅ Created data directory: ${DATA_DIR}"
fi

# Backup existing data files
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
if [ -f "${DATA_DIR}/arbitrage_data.csv" ]; then
    cp "${DATA_DIR}/arbitrage_data.csv" "${DATA_DIR}/arbitrage_data_backup_${TIMESTAMP}.csv"
    echo "💾 Backed up existing arbitrage data"
fi

echo ""
echo "🔥 Starting bot... Press Ctrl+C to stop early"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Run the bot with timeout (macOS compatible using gtimeout or perl)
if command -v gtimeout &> /dev/null; then
    # Use gtimeout if available (brew install coreutils)
    gtimeout ${DURATION_SECONDS}s cargo run --package engine --release || {
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 124 ]; then
            echo ""
            echo "⏰ Collection period completed (${DURATION_HOURS} hours)"
        else
            echo ""
            echo "⚠️  Bot exited with code: $EXIT_CODE"
        fi
    }
else
    # Fallback: run in background with kill after duration
    echo "⚠️  Note: Using background mode (install 'brew install coreutils' for better timeout support)"
    cargo run --package engine --release &
    BOT_PID=$!
    sleep ${DURATION_SECONDS}
    kill $BOT_PID 2>/dev/null || true
    wait $BOT_PID 2>/dev/null
    echo ""
    echo "⏰ Collection period completed (${DURATION_HOURS} hours)"
fi

# Show collection stats
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📈 Data Collection Summary:"

if [ -f "${DATA_DIR}/market_data.csv" ]; then
    POOL_UPDATES=$(wc -l < "${DATA_DIR}/market_data.csv")
    echo "  - Pool Updates: ${POOL_UPDATES} records"
fi

if [ -f "${DATA_DIR}/arbitrage_data.csv" ]; then
    ARB_OPPORTUNITIES=$(tail -n +2 "${DATA_DIR}/arbitrage_data.csv" | wc -l)
    echo "  - Arbitrage Opportunities: ${ARB_OPPORTUNITIES} records"
    
    if [ $ARB_OPPORTUNITIES -lt 100 ]; then
        echo ""
        echo "⚠️  WARNING: Less than 100 opportunities collected"
        echo "   Consider extending collection period or adding more pools"
    else
        echo "  ✅ Sufficient data collected for training"
    fi
fi

echo ""
echo "📁 Data files saved in: ${DATA_DIR}/"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo "  1. Review data: head -20 ${DATA_DIR}/arbitrage_data.csv"
echo "  2. Train model: python scripts/train_model.py"
echo "  3. Test new model: cargo run --package engine"
