#!/usr/bin/env bash
# Production Bot Monitor
# Watches bot logs for Success Library activity and performance

LOG_FILE=$1
if [ -z "$LOG_FILE" ]; then
    echo "Usage: $0 <log_file>"
    exit 1
fi

echo "🔍 Monitoring bot logs: $LOG_FILE"
echo "Looking for: Blacklist checks, Opportunities, Feedback loop activity"
echo "Press Ctrl+C to stop"
echo ""

tail -f "$LOG_FILE" | grep --line-buffered -E "FEEDBACK LOOP|is_blacklisted|SUCCESS LIBRARY|Opportunity|BUNDLE|blacklist|false positive|Intelligence" | while read line; do
    if [[ $line == *"FEEDBACK LOOP"* ]]; then
        echo "🔴 $line"
    elif [[ $line == *"blacklist"* ]]; then
        echo "⚠️  $line"
    elif [[ $line == *"Opportunity"* ]]; then
        echo "💰 $line"
    elif [[ $line == *"BUNDLE"* ]]; then
        echo "🚀 $line"
    else
        echo "ℹ️  $line"
    fi
done
