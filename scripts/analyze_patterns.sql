-- Success Library Pattern Analysis Queries
-- These queries help identify what makes tokens successful

-- Query 1: Liquidity Tier Analysis
-- Find relationship between initial liquidity and success
WITH liquidity_analysis AS (
    SELECT 
        CASE 
            WHEN liquidity_min < 10000 THEN 'Low (<10K)'
            WHEN liquidity_min < 50000 THEN 'Medium (10-50K)'
            WHEN liquidity_min < 100000 THEN 'High (50-100K)'
            ELSE 'Very High (>100K)'
        END as liquidity_tier,
        AVG(peak_roi) as avg_roi,
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY peak_roi) as median_roi,
        AVG(time_to_peak_secs) as avg_time_to_peak,
        COUNT(*) as sample_size,
        COUNT(*) FILTER (WHERE peak_roi > 200) as big_winners,
        COUNT(*) FILTER (WHERE is_false_positive) as failures
    FROM success_stories
    GROUP BY liquidity_tier
)
SELECT 
    liquidity_tier,
    ROUND(avg_roi::numeric, 2) as avg_roi_pct,
    ROUND(median_roi::numeric, 2) as median_roi_pct,
    ROUND((avg_time_to_peak / 60.0)::numeric, 1) as avg_mins_to_peak,
    sample_size,
    big_winners,
    ROUND((big_winners * 100.0 / NULLIF(sample_size, 0))::numeric, 1) as big_winner_rate_pct,
    failures,
    ROUND((failures * 100.0 / NULLIF(sample_size, 0))::numeric, 1) as failure_rate_pct
FROM liquidity_analysis
ORDER BY avg_roi DESC;

-- Query 2: Market Cap Analysis
-- Optimal entry market cap range
WITH mcap_analysis AS (
    SELECT 
        CASE 
            WHEN initial_market_cap < 50000 THEN 'Micro (<50K)'
            WHEN initial_market_cap < 100000 THEN 'Small (50-100K)'
            WHEN initial_market_cap < 500000 THEN 'Medium (100-500K)'
            ELSE 'Large (>500K)'
        END as mcap_tier,
        AVG(peak_roi) as avg_roi,
        AVG(drawdown) as avg_drawdown,
        COUNT(*) as count
    FROM success_stories
    WHERE is_false_positive = FALSE
    GROUP BY mcap_tier
)
SELECT 
    mcap_tier,
    ROUND(avg_roi::numeric, 2) as avg_roi_pct,
    ROUND(avg_drawdown::numeric, 2) as avg_drawdown_pct,
    count as successful_trades,
    ROUND((avg_roi - avg_drawdown)::numeric, 2) as risk_adjusted_roi
FROM mcap_analysis
ORDER BY risk_adjusted_roi DESC;

-- Query 3: Social Signal Analysis
-- Impact of Twitter presence
SELECT 
    has_twitter,
    COUNT(*) as total_tokens,
    AVG(peak_roi) as avg_roi,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY peak_roi) as median_roi,
    AVG(time_to_peak_secs / 60.0) as avg_mins_to_peak,
    COUNT(*) FILTER (WHERE peak_roi > 300) as moonshots,
    COUNT(*) FILTER (WHERE is_false_positive) as failures
FROM success_stories
GROUP BY has_twitter
ORDER BY avg_roi DESC;

-- Query 4: Mint Renounced Impact
-- Does renounced ownership correlate with success?
SELECT 
    mint_renounced,
    COUNT(*) as total,
    AVG(peak_roi) as avg_roi,
    STDDEV(peak_roi) as roi_volatility,
    COUNT(*) FILTER (WHERE is_false_positive) as failures,
    ROUND((COUNT(*) FILTER (WHERE is_false_positive) * 100.0 / COUNT(*))::numeric, 2) as failure_rate_pct
FROM success_stories
GROUP BY mint_renounced
ORDER BY avg_roi DESC;

-- Query 5: Time-based Pattern Analysis
-- When are the best tokens launching?
SELECT 
    EXTRACT(HOUR FROM created_at) as launch_hour_utc,
    COUNT(*) as launches,
    AVG(peak_roi) as avg_roi,
    COUNT(*) FILTER (WHERE peak_roi > 200) as big_winners
FROM success_stories
WHERE is_false_positive = FALSE
GROUP BY launch_hour_utc
ORDER BY avg_roi DESC
LIMIT 10;

-- Query 6: Success DNA - Composite Analysis
-- What characteristics do the top 10% performers share?
WITH top_performers AS (
    SELECT *
    FROM success_stories
    WHERE peak_roi > (
        SELECT PERCENTILE_CONT(0.9) WITHIN GROUP (ORDER BY peak_roi)
        FROM success_stories
        WHERE is_false_positive = FALSE
    )
    AND is_false_positive = FALSE
)
SELECT 
    AVG(liquidity_min) as avg_liquidity,
    AVG(initial_market_cap) as avg_mcap,
    COUNT(*) FILTER (WHERE has_twitter) * 100.0 / COUNT(*) as twitter_pct,
    COUNT(*) FILTER (WHERE mint_renounced) * 100.0 / COUNT(*) as renounced_pct,
    AVG(peak_roi) as avg_peak_roi,
    AVG(time_to_peak_secs / 60.0) as avg_mins_to_peak,
    COUNT(*) as sample_size,
    MIN(peak_roi) as min_roi_in_top_10pct,
    MAX(peak_roi) as max_roi
FROM top_performers;

-- Query 7: Failure Pattern Analysis
-- What characterizes false positives?
WITH failure_patterns AS (
    SELECT 
        AVG(liquidity_min) as avg_liquidity,
        AVG(initial_market_cap) as avg_mcap,
        AVG(peak_roi) as avg_peak_roi,
        AVG(drawdown) as avg_drawdown,
        COUNT(*) FILTER (WHERE has_twitter) * 100.0 / COUNT(*) as twitter_pct,
        COUNT(*) as total_failures
    FROM success_stories
    WHERE is_false_positive = TRUE
)
SELECT * FROM failure_patterns;

-- Query 8: Strategy Effectiveness Over Time
-- Is our strategy getting better?
SELECT 
    DATE_TRUNC('week', created_at) as week,
    COUNT(*) as trades,
    AVG(peak_roi) as avg_roi,
    COUNT(*) FILTER (WHERE is_false_positive) as failures,
    ROUND((COUNT(*) FILTER (WHERE NOT is_false_positive) * 100.0 / COUNT(*))::numeric, 2) as success_rate_pct
FROM success_stories
GROUP BY week
ORDER BY week DESC
LIMIT 12;

-- Query 9: Risk-Reward Matrix
-- Categorize tokens by risk/reward profile
SELECT 
    CASE 
        WHEN peak_roi > 300 AND drawdown < 20 THEN 'High Reward, Low Risk'
        WHEN peak_roi > 300 AND drawdown >= 20 THEN 'High Reward, High Risk'
        WHEN peak_roi <= 300 AND drawdown < 20 THEN 'Med Reward, Low Risk'
        ELSE 'Med Reward, High Risk'
    END as risk_reward_profile,
    COUNT(*) as count,
    AVG(liquidity_min) as avg_entry_liquidity,
    AVG(initial_market_cap) as avg_entry_mcap
FROM success_stories
WHERE is_false_positive = FALSE
GROUP BY risk_reward_profile
ORDER BY count DESC;

-- Query 10: Blacklist Review
-- What tokens are on the blacklist and why?
SELECT 
    token_address,
    strategy_id,
    peak_roi,
    drawdown,
    lesson,
    created_at
FROM success_stories
WHERE is_false_positive = TRUE
ORDER BY created_at DESC
LIMIT 20;
