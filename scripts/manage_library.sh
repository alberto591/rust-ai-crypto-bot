#!/usr/bin/env bash
# Success Library Management CLI
# Provides commands to manage the blacklist and view Success Library stats

set -e

DB_NAME="mev_bot_success_library"
PSQL="/opt/homebrew/opt/postgresql@17/bin/psql"

show_help() {
    cat << EOF
Success Library Management CLI

Usage: $0 <command> [arguments]

Commands:
    add-blacklist <token_address>       Add token to blacklist
    remove-blacklist <token_address>    Remove token from blacklist
    list-blacklist                      List all blacklisted tokens
    stats                               Show library statistics
    analyze                             Run DNA analysis
    auto-detect [threshold]             Auto-detect false positives (default: -80% ROI)

Examples:
    $0 add-blacklist 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R
    $0 list-blacklist
    $0 stats
    $0 auto-detect -70

EOF
}

add_blacklist() {
    local token=$1
    if [ -z "$token" ]; then
        echo "❌ Error: Token address required"
        exit 1
    fi
    
    echo "⚠️  Adding token to blacklist: $token"
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        $PSQL -d $DB_NAME -c "
            UPDATE success_stories 
            SET is_false_positive = TRUE 
            WHERE token_address = '$token';
        "
        echo "✅ Token blacklisted successfully"
    else
        echo "❌ Cancelled"
    fi
}

remove_blacklist() {
    local token=$1
    if [ -z "$token" ]; then
        echo "❌ Error: Token address required"
        exit 1
    fi
    
    echo "⚠️  Removing token from blacklist: $token"
    read -p "Are you sure? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        $PSQL -d $DB_NAME -c "
            UPDATE success_stories 
            SET is_false_positive = FALSE 
            WHERE token_address = '$token';
        "
        echo "✅ Token removed from blacklist"
    else
        echo "❌ Cancelled"
    fi
}

list_blacklist() {
    echo "🔴 Blacklisted Tokens:"
    $PSQL -d $DB_NAME -c "
        SELECT 
            token_address,
            strategy_id,
            peak_roi,
            drawdown,
            lesson
        FROM success_stories 
        WHERE is_false_positive = TRUE
        ORDER BY timestamp DESC;
    "
}

show_stats() {
    echo "📊 Success Library Statistics:"
    $PSQL -d $DB_NAME -c "
        SELECT 
            COUNT(*) as total_stories,
            SUM(CASE WHEN is_false_positive THEN 1 ELSE 0 END) as blacklisted,
            SUM(CASE WHEN NOT is_false_positive THEN 1 ELSE 0 END) as successful,
            AVG(CASE WHEN NOT is_false_positive THEN peak_roi ELSE NULL END)::numeric(10,2) as avg_success_roi,
            AVG(CASE WHEN is_false_positive THEN peak_roi ELSE NULL END)::numeric(10,2) as avg_failed_roi
        FROM success_stories;
    "
}

run_analysis() {
    echo "🧬 Running Success DNA Analysis..."
    cd "$(dirname "$0")/.."
    ./target/release/engine --analyze 2>&1 | grep -A 10 "SUCCESS LIBRARY ANALYSIS" || echo "Run 'cargo build --release' first"
}

auto_detect_false_positives() {
    local threshold=${1:--80}
    echo "🔍 Auto-detecting false positives (ROI threshold: ${threshold}%)..."
    
    $PSQL -d $DB_NAME -c "
        UPDATE success_stories 
        SET is_false_positive = TRUE 
        WHERE peak_roi < $threshold 
        AND is_false_positive = FALSE
        RETURNING token_address, peak_roi, lesson;
    "
    
    echo "✅ False positive detection complete"
}

# Main command dispatcher
case "${1:-help}" in
    add-blacklist)
        add_blacklist "$2"
        ;;
    remove-blacklist)
        remove_blacklist "$2"
        ;;
    list-blacklist|list)
        list_blacklist
        ;;
    stats)
        show_stats
        ;;
    analyze)
        run_analysis
        ;;
    auto-detect)
        auto_detect_false_positives "$2"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "❌ Unknown command: $1"
        show_help
        exit 1
        ;;
esac
