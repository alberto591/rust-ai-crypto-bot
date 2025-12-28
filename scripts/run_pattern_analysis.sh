#!/usr/bin/env bash
# Success Library Pattern Analyzer
# Runs all pattern analysis queries and generates insights

set -e

DB_NAME="mev_bot_success_library"
PSQL="/opt/homebrew/opt/postgresql@17/bin/psql"
REPORT_FILE="/tmp/pattern_analysis_$(date +%Y%m%d_%H%M%S).txt"

echo "🔍 Success Library Pattern Analysis"
echo "======================================"
echo "Generated: $(date)"
echo "Database: $DB_NAME"
echo ""
echo "Report will be saved to: $REPORT_FILE"
echo ""

# Run all analysis queries
$PSQL -d $DB_NAME -f scripts/analyze_patterns.sql > $REPORT_FILE 2>&1

# Display summary
cat $REPORT_FILE

echo ""
echo "✅ Analysis complete!"
echo "📄 Full report: $REPORT_FILE"
echo ""
echo "🎯 Key Insights:"

# Extract key metrics
echo "1. Best Liquidity Tier:"
$PSQL -d $DB_NAME -t -c "
WITH liquidity_analysis AS (
    SELECT 
        CASE 
            WHEN liquidity_min < 10000 THEN 'Low'
            WHEN liquidity_min < 50000 THEN 'Medium'
            ELSE 'High'
        END as tier,
        AVG(peak_roi) as avg_roi
    FROM success_stories
    WHERE is_false_positive = FALSE
    GROUP BY tier
)
SELECT tier || ': ' || ROUND(avg_roi::numeric, 2) || '% avg ROI'
FROM liquidity_analysis
ORDER BY avg_roi DESC
LIMIT 1;
"

echo "2. Twitter Impact:"
$PSQL -d $DB_NAME -t -c "
SELECT 
    'With Twitter: ' || ROUND(AVG(CASE WHEN has_twitter THEN peak_roi END)::numeric, 2) || '% | ' ||
    'Without: ' || ROUND(AVG(CASE WHEN NOT has_twitter THEN peak_roi END)::numeric, 2) || '%'
FROM success_stories
WHERE is_false_positive = FALSE;
"

echo "3. Current Success Rate:"
$PSQL -d $DB_NAME -t -c "
SELECT ROUND((COUNT(*) FILTER (WHERE NOT is_false_positive) * 100.0 / COUNT(*))::numeric, 1) || '% (' ||
       COUNT(*) FILTER (WHERE NOT is_false_positive) || '/' || COUNT(*) || ' trades)'
FROM success_stories;
"

echo "4. Blacklist Size:"
$PSQL -d $DB_NAME -t -c "
SELECT COUNT(*) || ' tokens blacklisted'
FROM success_stories
WHERE is_false_positive = TRUE;
"

echo ""
echo "💡 Recommendations:"

# Generate recommendations based on data
BEST_HOUR=$($PSQL -d $DB_NAME -t -c "
SELECT EXTRACT(HOUR FROM created_at)::integer
FROM success_stories
WHERE is_false_positive = FALSE
GROUP BY EXTRACT(HOUR FROM created_at)
ORDER BY AVG(peak_roi) DESC
LIMIT 1;
" | tr -d ' ')

if [ ! -z "$BEST_HOUR" ]; then
    echo "- Best launch hour: ${BEST_HOUR}:00 UTC"
fi

echo "- Review full report for detailed insights"
echo "- Run weekly to track strategy evolution"
echo ""
