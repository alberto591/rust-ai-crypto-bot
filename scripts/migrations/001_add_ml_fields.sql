-- Migration: Add Enhanced Learning Fields to success_stories
-- Phase 6: External Intelligence Integration

ALTER TABLE success_stories
ADD COLUMN IF NOT EXISTS holder_count_at_peak BIGINT,
ADD COLUMN IF NOT EXISTS market_volatility DOUBLE PRECISION,
ADD COLUMN IF NOT EXISTS launch_hour_utc SMALLINT;

-- Add comments for documentation
COMMENT ON COLUMN success_stories.holder_count_at_peak IS 'Number of unique token holders at peak ROI';
COMMENT ON COLUMN success_stories.market_volatility IS 'BTC/ETH volatility index at time of trade';
COMMENT ON COLUMN success_stories.launch_hour_utc IS 'UTC hour (0-23) when token launched';

-- Create index for time-based analysis
CREATE INDEX IF NOT EXISTS idx_success_stories_launch_hour ON success_stories(launch_hour_utc) WHERE launch_hour_utc IS NOT NULL;

-- Verify migration
SELECT column_name, data_type, is_nullable 
FROM information_schema.columns 
WHERE table_name = 'success_stories'
ORDER BY ordinal_position;
