use bytemuck::{Pod, Zeroable};
use solana_sdk::pubkey::Pubkey;

/// Meteora DLMM (Dynamic Liquidity Market Maker) pool structure
/// Uses bin-based liquidity for concentrated liquidity
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MeteoraDLMM {
    pub data: [u8; 1024],
}

unsafe impl Zeroable for MeteoraDLMM {}
unsafe impl Pod for MeteoraDLMM {}

impl MeteoraDLMM {
    #[inline(always)]
    pub fn token_x_mint(&self) -> Pubkey {
        // Offset for token X mint (needs to be verified with actual Meteora layout)
        Pubkey::new_from_array(self.data[8..40].try_into().unwrap())
    }

    #[inline(always)]
    pub fn token_y_mint(&self) -> Pubkey {
        // Offset for token Y mint
        Pubkey::new_from_array(self.data[40..72].try_into().unwrap())
    }

    #[inline(always)]
    pub fn active_bin_id(&self) -> i32 {
        // Current active bin ID
        i32::from_le_bytes(self.data[72..76].try_into().unwrap())
    }

    #[inline(always)]
    pub fn bin_step(&self) -> u16 {
        // Price step between bins in basis points
        u16::from_le_bytes(self.data[76..78].try_into().unwrap())
    }

    #[inline(always)]
    pub fn base_fee_rate(&self) -> u16 {
        // Base fee in basis points
        u16::from_le_bytes(self.data[78..80].try_into().unwrap())
    }

    /// Calculate price from bin ID
    /// Price = (1 + bin_step/10000)^bin_id
    pub fn calculate_price_from_bin(&self, bin_id: i32) -> f64 {
        let bin_step = self.bin_step() as f64 / 10000.0;
        (1.0 + bin_step).powi(bin_id)
    }

    /// Get current pool price
    pub fn get_current_price(&self) -> f64 {
        let active_bin = self.active_bin_id();
        self.calculate_price_from_bin(active_bin)
    }

    /// Estimate swap output (simplified - real implementation needs bin traversal)
    pub fn estimate_swap_output(
        &self,
        amount_in: u64,
        x_to_y: bool,
    ) -> Result<u64, &'static str> {
        let price = self.get_current_price();
        let fee_rate = self.base_fee_rate();
        
        // Apply fee
        let amount_in_after_fee = amount_in as u128 * (10000 - fee_rate as u128) / 10000;
        
        let amount_out = if x_to_y {
            (amount_in_after_fee as f64 * price) as u64
        } else {
            (amount_in_after_fee as f64 / price) as u64
        };
        
        Ok(amount_out)
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MeteoraSwapKeys {
    pub dlmm_pool: Pubkey,
    pub bin_array_bitmap_extension: Option<Pubkey>,
    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,
    pub token_x_mint: Pubkey,
    pub token_y_mint: Pubkey,
    pub oracle: Pubkey,
    pub user_token_x: Pubkey,
    pub user_token_y: Pubkey,
}
