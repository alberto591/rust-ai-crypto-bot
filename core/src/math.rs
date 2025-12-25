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
    _liquidity: u128, // Currently using price-based approximation for speed
    fee_bps: u16,
    a_to_b: bool,
) -> u64 {
    if amount_in == 0 || sqrt_price_x64 == 0 {
        return 0;
    }

    // Convert X64 sqrt price to price
    // Price = (sqrt_price / 2^64)^2  (This is token_b / token_a)
    let sqrt_p = sqrt_price_x64 as f64 / (1u128 << 64) as f64;
    let price = if a_to_b {
        sqrt_p * sqrt_p
    } else {
        1.0 / (sqrt_p * sqrt_p)
    };

    let fee_multiplier = 1.0 - (fee_bps as f64 / 10000.0);
    let amount_in_with_fee = amount_in as f64 * fee_multiplier;

    // dy = dx * price
    (amount_in_with_fee * price) as u64
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
    fn test_effective_price() {
        let amount_in = 1_000_000u64;
        let amount_out = 950_000u64;
        let price = calculate_effective_price(amount_in, amount_out);
        assert_eq!(price, 0.95);
    }
}
