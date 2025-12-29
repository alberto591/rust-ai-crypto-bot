#!/bin/bash

# Diagnostic script for Mojito Solana MEV Bot
# Finds "Logic Leaks" in the trade pipeline

LOG_FILE=$(ls -rt logs/production_2hour_*.log | tail -n 1)

if [ -z "$LOG_FILE" ]; then
    echo "❌ No production log found in logs/"
    exit 1
fi

echo "--- 🔍 MOJITO LOGIC LEAK DIAGNOSTIC ---"
echo "Log: $LOG_FILE"
echo "Time: $(date)"
echo "---------------------------------------"

# 1. Detection Leak: Are we seeing events?
echo "📡 [DETECTION]"
echo "Total New Pools Detected: $(grep -c "New Pool Detected" "$LOG_FILE")"
echo "Total Pump.fun Hydrations: $(grep -c "Hydrating Pump.fun" "$LOG_FILE")"
echo "Total Raydium Discoveries: $(grep -c "Raydium V4 Discovery" "$LOG_FILE")"

# 2. Filter Leak: Are we rejecting too much?
echo -e "\n🛡️ [FILTER REJECTIONS]"
echo "Total Arbitrage Opps Found: $(grep -c "ARB_FOUND" "$LOG_FILE")"
echo "Rejected by Sanity (Profit): $(grep -c "Rejected by Sanity" "$LOG_FILE")"
echo "Rejected by Safety Checker: $(grep -c "Rejected by Safety Checker" "$LOG_FILE")"
echo "Rejected by DNA Gate: $(grep -c "DNA GATE: Token does not match" "$LOG_FILE")"

# 3. Execution Leak: Are we landing?
echo -e "\n🔥 [EXECUTION]"
echo "Bundles Dispatched: $(grep -c "BUNDLE DISPATCHED" "$LOG_FILE")"
echo "Trades Confirmed: $(grep -c "💰 Trade Confirmed" "$LOG_FILE")"
echo "Trades Failed On-Chain: $(grep -c "💸 Trade Failed on-chain" "$LOG_FILE")"
echo "Jito Endpoint Failures: $(grep -c "❌ Jito endpoint.*exhausted all" "$LOG_FILE")"

# 4. Hardware/Latency Check
echo -e "\n⚡ [LATENCY]"
echo "Average Hydration Time (Estimated): $(grep "Hydration took:" "$LOG_FILE" | awk '{sum+=$NF; count++} END {if (count > 0) print sum/count "ms"; else print "N/A"}')"
echo "Average Strategy Time: $(grep "Strategy evaluation took:" "$LOG_FILE" | awk '{sum+=$NF; count++} END {if (count > 0) print sum/count "ms"; else print "N/A"}')"

echo "---------------------------------------"
echo "💡 RECOMMENDATION:"
if [ $(grep -c "BUNDLE DISPATCHED" "$LOG_FILE") -eq 0 ]; then
    if [ $(grep -c "ARB_FOUND" "$LOG_FILE") -gt 0 ]; then
        echo "👉 Critical Filter Leak: We are seeing opportunities but rejecting all of them. Check your MIN_LIQUIDITY and SANITY_PROFIT_FACTOR."
    else
        echo "👉 Critical Detection Leak: We aren't finding arbitrage. Ensure multiple DEXs are monitored or check RPC subscription health."
    fi
fi
