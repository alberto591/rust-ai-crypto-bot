use bytemuck::{Pod, Zeroable};
use solana_sdk::pubkey::Pubkey;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct AmmInfo {
    pub data: [u8; 752],
}

unsafe impl Zeroable for AmmInfo {}
unsafe impl Pod for AmmInfo {}

impl AmmInfo {
    #[inline(always)]
    pub fn base_mint(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[400..432].try_into().unwrap())
    }

    #[inline(always)]
    pub fn quote_mint(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[432..464].try_into().unwrap())
    }

    #[inline(always)]
    pub fn base_vault(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[336..368].try_into().unwrap())
    }

    #[inline(always)]
    pub fn quote_vault(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[368..400].try_into().unwrap())
    }

    #[inline(always)]
    pub fn lp_mint(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[464..496].try_into().unwrap())
    }

    #[inline(always)]
    pub fn open_orders(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[496..528].try_into().unwrap())
    }

    #[inline(always)]
    pub fn target_orders(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[592..624].try_into().unwrap())
    }

    #[inline(always)]
    pub fn market_id(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[528..560].try_into().unwrap())
    }

    #[inline(always)]
    pub fn market_program_id(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[560..592].try_into().unwrap())
    }

    #[inline(always)]
    pub fn base_reserve(&self) -> u64 {
        u64::from_le_bytes(self.data[720..728].try_into().unwrap())
    }

    #[inline(always)]
    pub fn quote_reserve(&self) -> u64 {
        u64::from_le_bytes(self.data[728..736].try_into().unwrap())
    }
}

/// All account keys required for a Raydium V4 swap
/// Order is CRITICAL - must match Raydium program expectations exactly
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RaydiumSwapKeys {
    pub amm_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_target_orders: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub serum_program_id: Pubkey,
    pub serum_market: Pubkey,
    pub serum_bids: Pubkey,
    pub serum_asks: Pubkey,
    pub serum_event_queue: Pubkey,
    pub serum_coin_vault: Pubkey,
    pub serum_pc_vault: Pubkey,
    pub serum_vault_signer: Pubkey,
    pub user_source_token_account: Pubkey,
    pub user_dest_token_account: Pubkey,
    pub user_owner: Pubkey,
    pub token_program: Pubkey,
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
