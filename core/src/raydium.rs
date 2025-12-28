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

/// Serum V3 / OpenBook Market Layout (388 bytes)
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MarketStateV3 {
    pub data: [u8; 388],
}

unsafe impl Zeroable for MarketStateV3 {}
unsafe impl Pod for MarketStateV3 {}

impl MarketStateV3 {
    #[inline(always)]
    pub fn bids(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[285..317].try_into().unwrap())
    }

    #[inline(always)]
    pub fn asks(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[317..349].try_into().unwrap())
    }

    #[inline(always)]
    pub fn event_queue(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[253..285].try_into().unwrap())
    }

    #[inline(always)]
    pub fn coin_vault(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[117..149].try_into().unwrap())
    }

    #[inline(always)]
    pub fn pc_vault(&self) -> Pubkey {
        Pubkey::new_from_array(self.data[165..197].try_into().unwrap())
    }

    #[inline(always)]
    pub fn vault_signer_nonce(&self) -> u32 {
        u32::from_le_bytes(self.data[45..49].try_into().unwrap())
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
        // Create a dummy byte array of the correct size
        let mut data = [0u8; 752];

        // Set base_reserve at offset 720..728 (little endian)
        let base_reserve = 1000u64 * 10u64.pow(9); // 1000 SOL
        data[720..728].copy_from_slice(&base_reserve.to_le_bytes());

        // Set quote_reserve at offset 728..736
        let quote_reserve = 20000u64 * 10u64.pow(6); // 20000 USDC
        data[728..736].copy_from_slice(&quote_reserve.to_le_bytes());

        // Decode
        let decoded = AmmInfo { data };

        assert_eq!(decoded.base_reserve(), base_reserve);
        assert_eq!(decoded.quote_reserve(), quote_reserve);

        // Test price calculation logic (assuming 9 decimals for base, 6 for quote)
        let price = (decoded.quote_reserve() as f64 / 10f64.powi(6)) /
                    (decoded.base_reserve() as f64 / 10f64.powi(9));

        assert_eq!(price, 20.0);
    }
}
