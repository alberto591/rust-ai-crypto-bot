import pandas as pd
import numpy as np

def analyze_market():
    # Load market data
    # format: timestamp,pool_address,program_id,reserve_a,reserve_b,price_ratio
    try:
        df = pd.read_csv('data/market_data.csv', names=['timestamp', 'pool_address', 'program_id', 'reserve_a', 'reserve_b', 'price_ratio'])
    except Exception as e:
        print(f"Error loading market data: {e}")
        return

    # Group by pool_address to calculate volatility per pool
    pools = df['pool_address'].unique()
    print(f"Analyzing {len(pools)} pools...")

    results = []
    for pool in pools:
        pool_df = df[df['pool_address'] == pool].sort_values('timestamp')
        if len(pool_df) < 10:
            continue
        
        # Calculate returns
        pool_df['return'] = pool_df['price_ratio'].pct_change()
        volatility = pool_df['return'].std() * np.sqrt(len(pool_df)) # simple annualized-ish volatility
        
        # Max/Min price
        max_price = pool_df['price_ratio'].max()
        min_price = pool_df['price_ratio'].min()
        spread = (max_price - min_price) / min_price if min_price > 0 else 0
        
        results.append({
            'pool': pool,
            'volatility': volatility,
            'spread': spread,
            'count': len(pool_df)
        })

    results_df = pd.DataFrame(results).sort_values('volatility', ascending=False)
    print("\nTop 5 Volatile Pools:")
    print(results_df.head(5))

    avg_vol = results_df['volatility'].mean()
    print(f"\nAverage Volatility: {avg_vol:.6f}")
    
    # Recommendation
    if avg_vol > 0.05:
        print("Market is highly volatile. Recommend increasing profit threshold.")
    elif avg_vol > 0.02:
        print("Market is moderately volatile. Maintain current thresholds.")
    else:
        print("Market is stable. Consider lowering thresholds for more volume.")

if __name__ == "__main__":
    analyze_market()
