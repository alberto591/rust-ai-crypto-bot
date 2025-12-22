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
    pub fee_numerator: u64,
    pub fee_denominator: u64,
    // We store reserves to calculate exact output locally
    // without asking the RPC every time.
    pub reserve_in: u128,  // Reserve of 'from_token'
    pub reserve_out: u128, // Reserve of 'to_token'
}

pub struct MarketGraph {
    // Adjacency List: From Token -> List of Connections
    pub adj: HashMap<Pubkey, Vec<Edge>>,
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
        reserve_from: u64,
        reserve_to: u64,
    ) {
        let entry = self.adj.entry(from).or_insert(Vec::new());
        
        // Check if edge exists to update it (Fast Scan)
        if let Some(edge) = entry.iter_mut().find(|e| e.pool_address == pool) {
            edge.reserve_in = reserve_from as u128;
            edge.reserve_out = reserve_to as u128;
        } else {
            // New connection discovered
            entry.push(Edge {
                to_token: to,
                pool_address: pool,
                fee_numerator: 25, // Default Raydium 0.25%
                fee_denominator: 10000,
                reserve_in: reserve_from as u128,
                reserve_out: reserve_to as u128,
            });
        }
    }

    /// Calculates how much 'to_token' you get for 'amount_in'
    /// Uses Constant Product Formula: dy = (y * dx) / (x + dx)
    /// (Simplified version ignoring complicated fee math for brevity)
    pub fn get_amount_out(&self, edge: &Edge, amount_in: u64) -> u64 {
        let amount_in_u128 = amount_in as u128;
        
        // Apply Fee (Input amount * (1 - fee))
        let fee_multiplier = edge.fee_denominator as u128 - edge.fee_numerator as u128;
        let amount_in_with_fee = amount_in_u128 * fee_multiplier;
        
        let numerator = amount_in_with_fee * edge.reserve_out;
        let denominator = (edge.reserve_in * edge.fee_denominator as u128) + amount_in_with_fee;

        if denominator == 0 { return 0; }
        (numerator / denominator) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_graph_update_and_calc() {
        let mut graph = MarketGraph::new();
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let pool = Pubkey::new_unique();

        // 1. Add edge: 1000 A <-> 2000 B
        graph.update_edge(token_a, token_b, pool, 1000, 2000);

        // 2. Calculate amount out for 10 A
        // CPMM: dy = (2000 * (10*0.9975)) / (1000 + 10*0.9975)
        let edge = &graph.adj[&token_a][0];
        let amount_out = graph.get_amount_out(edge, 10);
        
        assert!(amount_out > 0);
        assert!(amount_out < 20); // Should be slightly less than 20 due to reserves ratio and fees
    }
}
