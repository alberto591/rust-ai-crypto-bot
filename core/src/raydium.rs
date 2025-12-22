use bytemuck::{Pod, Zeroable};
use solana_sdk::pubkey::Pubkey;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct AmmInfo {
    pub status: u64,
    pub nonce: u64,
    pub order_num: u64,
    pub depth: u64,
    pub base_decimals: u64,
    pub quote_decimals: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave_ratio: u64,
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub system_decimal_value: u64,
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_fee_numerator: u64,
    pub pnl_fee_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub base_need_take_pnl: u64,
    pub quote_need_take_pnl: u64,
    pub quote_total_pnl: u64,
    pub base_total_pnl: u64,
    pub pool_open_time: u64,
    pub punish_pc_amount: u64,
    pub punish_coin_amount: u64,
    pub orderbook_to_init_time: u64,
    pub swap_base_in_amount: [u64; 2],
    pub swap_quote_out_amount: [u64; 2],
    pub swap_base2quote_fee: u64,
    pub swap_quote_in_amount: [u64; 2],
    pub swap_base_out_amount: [u64; 2],
    pub swap_quote2base_fee: u64,
    // Vaults
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market_id: Pubkey,
    pub market_program_id: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub lp_vault: Pubkey,
    pub amm_owner: Pubkey,
    pub lp_reserve: u64,
    pub padding: [u64; 3],
    // Reserves
    pub base_reserve: u64,
    pub quote_reserve: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amm_info_decoding() {
        // Create a dummy byte array of the correct size (at least 752 bytes for a full V4 layout, 
        // but AmmInfo as defined is ~456 bytes + scaling)
        let mut data = vec![0u8; std::mem::size_of::<AmmInfo>()];
        
        let mut amm = AmmInfo::zeroed();
        amm.status = 1;
        amm.base_decimals = 9;
        amm.quote_decimals = 6;
        amm.base_reserve = 1000 * 10u64.pow(9); // 1000 SOL
        amm.quote_reserve = 20000 * 10u64.pow(6); // 20000 USDC
        
        // Copy struct bytes to data
        let bytes = bytemuck::bytes_of(&amm);
        data[..bytes.len()].copy_from_slice(bytes);
        
        // Decode back
        let decoded = bytemuck::try_from_bytes::<AmmInfo>(&data).unwrap();
        
        assert_eq!(decoded.status, 1);
        assert_eq!(decoded.base_reserve, 1000 * 10u64.pow(9));
        
        // Test price calculation logic
        let price = (decoded.quote_reserve as f64 / 10f64.powi(decoded.quote_decimals as i32)) /
                    (decoded.base_reserve as f64 / 10f64.powi(decoded.base_decimals as i32));
        
        assert_eq!(price, 20.0);
    }
}
