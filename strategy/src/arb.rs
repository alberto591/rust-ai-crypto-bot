/// Arbitrage Search Engine
/// 
/// Performs Depth First Search (DFS) to find profitable cycles in the market graph.
/// Focusing on 3-hop cycles (Triangular Arbitrage): A -> B -> C -> A
use solana_sdk::pubkey::Pubkey;
use crate::graph::{MarketGraph, Edge};

#[derive(Debug, Clone)]
pub struct SwapPath {
    pub hops: Vec<Edge>,
    pub expected_profit: i64, // Can be negative
}

pub struct ArbFinder;

impl ArbFinder {
    /// Finds the best 3-hop cycle starting from `start_token`
    /// 
    /// # Arguments
    /// * `graph` - The market graph state
    /// * `start_token` - The base token to arb (e.g., SOL or USDC)
    /// * `amount_in` - Initial input amount
    /// 
    /// # Returns
    /// The most profitable `SwapPath` found, or None if no valid cycle exists.
    pub fn find_best_cycle(
        graph: &MarketGraph,
        start_token: Pubkey,
        amount_in: u64,
    ) -> Option<SwapPath> {
        let mut best_path: Option<SwapPath> = None;
        let mut max_profit = 0_i64;

        // Step 1: Find all neighbors of Start (Hop 1: Start -> B)
        if let Some(edges_1) = graph.adj.get(&start_token) {
            for edge_1 in edges_1 {
                let amt_1 = graph.get_amount_out(edge_1, amount_in);
                if amt_1 == 0 { continue; }

                // Step 2: Find neighbors of Hop 1 (Hop 2: B -> C)
                if let Some(edges_2) = graph.adj.get(&edge_1.to_token) {
                    for edge_2 in edges_2 {
                        // Optimization: Don't go back to start immediately (A->B->A is just a sandwich/ping-pong)
                        if edge_2.to_token == start_token { continue; }

                        let amt_2 = graph.get_amount_out(edge_2, amt_1);
                        if amt_2 == 0 { continue; }

                        // Step 3: Find path back to Start (Hop 3: C -> Start)
                        if let Some(edges_3) = graph.adj.get(&edge_2.to_token) {
                            for edge_3 in edges_3 {
                                if edge_3.to_token == start_token {
                                    let final_amt = graph.get_amount_out(edge_3, amt_2);
                                    
                                    let profit = final_amt as i64 - amount_in as i64;
                                    
                                    // Found a profitable path?
                                    if profit > max_profit {
                                        max_profit = profit;
                                        best_path = Some(SwapPath {
                                            hops: vec![edge_1.clone(), edge_2.clone(), edge_3.clone()],
                                            expected_profit: profit,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        best_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_cycle() {
        let mut graph = MarketGraph::new();
        
        let token_sol = Pubkey::new_unique();
        let token_usdc = Pubkey::new_unique();
        let token_bonk = Pubkey::new_unique();

        let pool_1 = Pubkey::new_unique();
        let pool_2 = Pubkey::new_unique();
        let pool_3 = Pubkey::new_unique();

        // Setup a profitable cycle:
        // 1. SOL -> USDC (Cheap USDC)
        graph.update_edge(token_sol, token_usdc, pool_1, mev_core::constants::RAYDIUM_V4_PROGRAM, 1_000_000_000, 100_000_000, None, None); 
        
        // 2. USDC -> BONK (Cheap BONK)
        graph.update_edge(token_usdc, token_bonk, pool_2, mev_core::constants::RAYDIUM_V4_PROGRAM, 100_000_000, 1_000_000_000_000, None, None);

        // 3. BONK -> SOL (Expensive SOL)
        // With these reserves, pumping 1 SOL in should get > 1 SOL out
        graph.update_edge(token_bonk, token_sol, pool_3, mev_core::constants::RAYDIUM_V4_PROGRAM, 1_000_000_000_000, 1_100_000_000, None, None); 

        // Run search with 1 SOL input
        let path = ArbFinder::find_best_cycle(&graph, token_sol, 1_000_000); // 0.001 SOL test
        
        // Should find a path
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.hops.len(), 3);
        assert!(p.expected_profit > 0);
    }
}
