-- Migration: Add pool_weights table
CREATE TABLE IF NOT EXISTS pool_weights (
    pool_address TEXT PRIMARY KEY,
    weight DOUBLE PRECISION NOT NULL DEFAULT 10.0,
    last_update_ts BIGINT NOT NULL,
    update_count INTEGER NOT NULL DEFAULT 0,
    dna_score INTEGER NOT NULL DEFAULT 0
);

-- Index for fast weight lookups
CREATE INDEX IF NOT EXISTS idx_pool_weights_value ON pool_weights (weight DESC);
