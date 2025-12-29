use serde::{Serialize, Deserialize};
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
pub struct PumpFunBondingCurve {
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

impl PumpFunBondingCurve {
    pub fn calculate_price_in_sol(&self) -> f64 {
        if self.virtual_token_reserves == 0 {
            return 0.0;
        }
        // Price = Virtual SOL / Virtual Token
        self.virtual_sol_reserves as f64 / self.virtual_token_reserves as f64
    }

    pub fn get_buy_price(&self, amount: u64) -> u64 {
        if self.virtual_token_reserves == 0 {
            return 0;
        }

        // k = x * y
        let k = self.virtual_sol_reserves as u128 * self.virtual_token_reserves as u128;
        
        // New Token Reserve = Virtual Token - Amount
        let new_virtual_token_reserves = (self.virtual_token_reserves as u128).saturating_sub(amount as u128);
        
        // New Sol Reserve = k / New Token Reserve
        let new_virtual_sol_reserves = k / new_virtual_token_reserves;
        
        // Cost = New Sol - Old Sol
        let cost = new_virtual_sol_reserves.saturating_sub(self.virtual_sol_reserves as u128);
        
        cost as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pump_fun_price() {
        // Standard Curve State (approximate)
        let curve = PumpFunBondingCurve {
            virtual_token_reserves: 1_000_000_000_000_000,
            virtual_sol_reserves: 30_000_000_000, // 30 SOL
            real_token_reserves: 800_000_000_000_000,
            real_sol_reserves: 0,
            token_total_supply: 1_000_000_000_000_000,
            complete: false,
        };

        let price = curve.calculate_price_in_sol();
        // Price should be 30 / 1_000_000_000 = 0.00000003 SOL per token
        assert!(price > 0.0);
        println!("Price: {:.12} SOL", price);
    }
}
