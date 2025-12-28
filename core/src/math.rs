/// Implementation of constant product market maker (CPMM) math for Raydium V4.
/// x * y = k
#[inline(always)]  // HFT: Force inline for hot path
pub fn get_amount_out_cpmm(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> u64 {
    if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
        return 0;
    }

    let fee_multiplier = 10000 - fee_bps as u128;
    let amount_in_with_fee = amount_in as u128 * fee_multiplier;
    
    let numerator = amount_in_with_fee * reserve_out as u128;
    let denominator = (reserve_in as u128 * 10000) + amount_in_with_fee;
    
    (numerator / denominator) as u64
}

/// Calculates the price impact percentage (0.0 to 1.0)
#[inline(always)]  // HFT: Force inline for hot path
pub fn calculate_price_impact(
    amount_in: u64,
    reserve_in: u64,
) -> f64 {
    if reserve_in == 0 { return 1.0; }
    amount_in as f64 / (reserve_in as f64 + amount_in as f64)
}

/// Calculates the effective price (amount_out / amount_in)
#[inline(always)]  // HFT: Force inline for hot path
pub fn calculate_effective_price(
    amount_in: u64,
    amount_out: u64,
) -> f64 {
    if amount_in == 0 { return 0.0; }
    amount_out as f64 / amount_in as f64
}

/// Placeholder for Concentrated Liquidity (CLMM) math (e.g., Orca Whirlpool).
/// This is significantly more complex and usually involves tick traversal.
/// Implementation of simplified CLMM math using virtual reserves for high-frequency discovery.
/// Note: This is an approximation. In production execution, exact tick-math should be used.
#[inline(always)]
pub fn get_amount_out_clmm(
    amount_in: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    fee_bps: u16,
    a_to_b: bool,
) -> u64 {
    if amount_in == 0 || sqrt_price_x64 == 0 || liquidity == 0 {
        return 0;
    }

    // 1. Calculate Virtual Reserves
    // L = sqrt(x * y), sqrt_p = sqrt(y / x)
    // x = L / sqrt_p, y = L * sqrt_p
    let sqrt_p = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
    
    let (v_res_in, v_res_out) = if a_to_b {
        // Selling A for B: res_in = x, res_out = y
        (liquidity as f64 / sqrt_p, liquidity as f64 * sqrt_p)
    } else {
        // Selling B for A: res_in = y, res_out = x
        (liquidity as f64 * sqrt_p, liquidity as f64 / sqrt_p)
    };

    // 2. Apply CPMM formula on virtual reserves
    let amount_in_f = amount_in as f64;
    let fee_multiplier = 1.0 - (fee_bps as f64 / 10000.0);
    let amount_in_with_fee = amount_in_f * fee_multiplier;

    let amount_out = (amount_in_with_fee * v_res_out) / (v_res_in + amount_in_with_fee);
    
    amount_out as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpmm_math() {
        let amount_in = 1_000_000u64; // 1 USDC
        let reserve_in = 1_000_000_000u64;
        let reserve_out = 2_000_000_000u64;
        let fee_bps = 30; // 0.3%

        let amount_out = get_amount_out_cpmm(amount_in, reserve_in, reserve_out, fee_bps);
        
        // Expected out: (1,000,000 * 0.997 * 2,000,000,000) / (1,000,000,000 + 1,000,000 * 0.997)
        // ~ 1,992,000
        assert!(amount_out > 1_900_000 && amount_out < 2_000_000);
    }

    #[test]
    fn test_price_impact() {
        let amount_in = 10_000_000u64; // 10% of 100M
        let reserve_in = 100_000_000u64;
        let impact = calculate_price_impact(amount_in, reserve_in);
        // impact = 10 / (100 + 10) = 10/110 = 0.0909...
        assert!(impact > 0.09 && impact < 0.10);
    }

    #[test]
    fn test_clmm_math_accurate() {
        let amount_in = 1_000_000u64; // 1 USDC
        let sqrt_price_x64: u128 = 18446744073709551616; // 1.0
        let liquidity: u128 = 1_000_000_000;
        let fee_bps = 30;

        // With 1.0 price and low liquidity, impact should be visible
        let amount_out = get_amount_out_clmm(amount_in, sqrt_price_x64, liquidity, fee_bps, true);
        
        // Price approx 1.0. 
        // Fee A: 1,000,000 * 0.997 = 997,000
        // Virtual Reserves: x = 1B, y = 1B
        // dy = (1B * 997k) / (1B + 997k) = 996,000ish
        assert!(amount_out < 997_000);
        assert!(amount_out > 990_000);
    }
}
