-- Success Library Schema (Phase 3)

CREATE TABLE IF NOT EXISTS success_stories (
    id SERIAL PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    token_address TEXT NOT NULL,
    market_context TEXT,
    lesson TEXT,
    timestamp BIGINT NOT NULL,
    
    -- Entry Triggers
    liquidity_min BIGINT,
    has_twitter BOOLEAN,
    mint_renounced BOOLEAN,
    initial_market_cap BIGINT,
    
    -- Performance Stats
    peak_roi DOUBLE PRECISION,
    time_to_peak_secs BIGINT,
    drawdown DOUBLE PRECISION,
    
    is_false_positive BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_success_stories_token ON success_stories(token_address);
CREATE INDEX IF NOT EXISTS idx_success_stories_strategy ON success_stories(strategy_id);

-- Performance-optimized indexes (Phase 4)
CREATE INDEX IF NOT EXISTS idx_success_stories_token_blacklist ON success_stories(token_address, is_false_positive);
CREATE INDEX IF NOT EXISTS idx_success_stories_blacklist ON success_stories(is_false_positive) WHERE is_false_positive = TRUE;
