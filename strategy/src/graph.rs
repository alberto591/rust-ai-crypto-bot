/// The Market Graph ("The Brain")
/// 
/// A custom directed graph implementation optimized for high-frequency arbitrage.
/// Models tokens as nodes and liquidity pools as edges.
/// 
/// Performance:
/// - O(1) Adjacency Lookups via HashMap
/// - Local reserve tracking for zero-RPC price calculation
/// - Allocation-optimized for rapid updates
use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct Edge {
    pub to_token: Pubkey,
    pub pool_address: Pubkey,
    pub program_id: Pubkey, // Added for DEX identification
    pub fee_numerator: u64,
    pub fee_denominator: u64,
    // CPMM Reserves (Raydium)
    pub reserve_in: u128,
    pub reserve_out: u128,
    // CLMM Data (Orca)
    pub price_sqrt: Option<u128>,
    pub liquidity: Option<u128>,
}

pub struct MarketGraph {
    // Adjacency List: From Token -> List of Connections
    pub adj: HashMap<Pubkey, Vec<Edge>>,
}

impl Default for MarketGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketGraph {
    pub fn new() -> Self {
        Self {
            adj: HashMap::new(),
        }
    }

    /// Adds or updates a connection between two tokens via a pool
    pub fn update_edge(
        &mut self,
        from: Pubkey,
        to: Pubkey,
        pool: Pubkey,
        program_id: Pubkey,
        reserve_from: u64,
        reserve_to: u64,
        price_sqrt: Option<u128>,
        liquidity: Option<u128>,
    ) {
        let edges = self.adj.entry(from).or_default();
        
        // Check if edge exists to update it (Fast Scan)
        if let Some(edge) = edges.iter_mut().find(|e| e.pool_address == pool) {
            edge.reserve_in = reserve_from as u128;
            edge.reserve_out = reserve_to as u128;
            edge.price_sqrt = price_sqrt;
            edge.liquidity = liquidity;
        } else {
            // New connection discovered
            edges.push(Edge {
                to_token: to,
                pool_address: pool,
                program_id,
                fee_numerator: 25, // Default. Should be dynamic based on DEX
                fee_denominator: 10000,
                reserve_in: reserve_from as u128,
                reserve_out: reserve_to as u128,
                price_sqrt,
                liquidity,
            });
        }
    }

    /// Calculates how much 'to_token' you get for 'amount_in'
    pub fn get_amount_out(&self, edge: &Edge, amount_in: u64) -> u64 {
        if edge.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM {
            if let Some(price_sqrt) = edge.price_sqrt {
                let liquidity = edge.liquidity.unwrap_or(0);
                let a_to_b = edge.reserve_in > edge.reserve_out; // Heuristic for direction in graph
                return mev_core::math::get_amount_out_clmm(amount_in, price_sqrt, liquidity, edge.fee_numerator as u128 as u16, a_to_b);
            }
            0
        } else {
            // Standard CPMM (Raydium)
            let amount_in_u128 = amount_in as u128;
            let fee_multiplier = edge.fee_denominator as u128 - edge.fee_numerator as u128;
            let amount_in_with_fee = amount_in_u128 * fee_multiplier;
            
            let numerator = amount_in_with_fee * edge.reserve_out;
            let denominator = (edge.reserve_in * edge.fee_denominator as u128) + amount_in_with_fee;

            if denominator == 0 { return 0; }
            (numerator / denominator) as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_update_and_calc() {
        let mut graph = MarketGraph::new();
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let pool = Pubkey::new_unique();

        // 1. Add edge: 1000 A <-> 2000 B
        graph.update_edge(token_a, token_b, pool, mev_core::constants::RAYDIUM_V4_PROGRAM, 1000, 2000, None, None);

        // 2. Calculate amount out for 10 A
        // CPMM: dy = (2000 * (10*0.9975)) / (1000 + 10*0.9975)
        let edge = &graph.adj[&token_a][0];
        let amount_out = graph.get_amount_out(edge, 10);
        
        assert!(amount_out > 0);
        assert!(amount_out < 20); // Should be slightly less than 20 due to reserves ratio and fees
    }
}
