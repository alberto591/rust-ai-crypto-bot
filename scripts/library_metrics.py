#!/usr/bin/env python3
"""
Success Library Metrics Reporter
Generates Prometheus-compatible metrics for the Success Library
"""

import subprocess
import sys
from datetime import datetime

DB_NAME = "mev_bot_success_library"
PSQL_PATH = "/opt/homebrew/opt/postgresql@17/bin/psql"

def run_query(sql):
    """Execute SQL query and return results"""
    try:
        result = subprocess.run(
            [PSQL_PATH, "-d", DB_NAME, "-t", "-c", sql],
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"Error executing query: {e}", file=sys.stderr)
        return None

def get_metrics():
    """Fetch all Success Library metrics"""
    metrics = {}
    
    # Total stories
    result = run_query("SELECT COUNT(*) FROM success_stories;")
    metrics['total_stories'] = int(result) if result else 0
    
    # Blacklisted count
    result = run_query("SELECT COUNT(*) FROM success_stories WHERE is_false_positive = TRUE;")
    metrics['blacklisted_count'] = int(result) if result else 0
    
    # Average ROI for successful trades
    result = run_query("SELECT AVG(peak_roi) FROM success_stories WHERE is_false_positive = FALSE;")
    metrics['avg_success_roi'] = float(result) if result and result != '' else 0.0
    
    # Median time to peak
    result = run_query("SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY time_to_peak_secs) FROM success_stories WHERE is_false_positive = FALSE;")
    metrics['median_time_to_peak'] = float(result) if result and result != '' else 0.0
    
    # Stories added in last 24h
    result = run_query("SELECT COUNT(*) FROM success_stories WHERE created_at > NOW() - INTERVAL '24 hours';")
    metrics['stories_24h'] = int(result) if result else 0
    
    # Blacklist hit rate (needs to be tracked separately in production)
    metrics['blacklist_hit_rate'] = 0.0  # Placeholder
    
    return metrics

def print_prometheus_metrics(metrics):
    """Output metrics in Prometheus format"""
    print("# HELP success_library_total Total number of success stories")
    print("# TYPE success_library_total gauge")
    print(f"success_library_total {metrics['total_stories']}")
    print()
    
    print("# HELP success_library_blacklisted Number of blacklisted tokens")
    print("# TYPE success_library_blacklisted gauge")
    print(f"success_library_blacklisted {metrics['blacklisted_count']}")
    print()
    
    print("# HELP success_library_avg_roi Average ROI of successful trades (%)")
    print("# TYPE success_library_avg_roi gauge")
    print(f"success_library_avg_roi {metrics['avg_success_roi']:.2f}")
    print()
    
    print("# HELP success_library_median_time_to_peak Median time to peak (seconds)")
    print("# TYPE success_library_median_time_to_peak gauge")
    print(f"success_library_median_time_to_peak {metrics['median_time_to_peak']:.0f}")
    print()
    
    print("# HELP success_library_stories_24h Stories added in last 24 hours")
    print("# TYPE success_library_stories_24h gauge")
    print(f"success_library_stories_24h {metrics['stories_24h']}")
    print()
    
    print("# HELP success_library_blacklist_hit_rate Blacklist check hit rate")
    print("# TYPE success_library_blacklist_hit_rate gauge")
    print(f"success_library_blacklist_hit_rate {metrics['blacklist_hit_rate']:.4f}")

def print_human_readable(metrics):
    """Output metrics in human-readable format"""
    print("=" * 60)
    print(f"SUCCESS LIBRARY METRICS REPORT")
    print(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 60)
    print(f"Total Stories:           {metrics['total_stories']:,}")
    print(f"Blacklisted Tokens:      {metrics['blacklisted_count']:,}")
    print(f"Successful Trades:       {metrics['total_stories'] - metrics['blacklisted_count']:,}")
    print(f"Average Success ROI:     {metrics['avg_success_roi']:.2f}%")
    print(f"Median Time to Peak:     {metrics['median_time_to_peak']:.0f}s")
    print(f"Stories (Last 24h):      {metrics['stories_24h']:,}")
    print(f"Blacklist Hit Rate:      {metrics['blacklist_hit_rate']:.2%}")
    print("=" * 60)

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="Success Library Metrics Reporter")
    parser.add_argument("--prometheus", action="store_true", help="Output in Prometheus format")
    args = parser.parse_args()
    
    metrics = get_metrics()
    
    if args.prometheus:
        print_prometheus_metrics(metrics)
    else:
        print_human_readable(metrics)
