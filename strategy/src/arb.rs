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
    /// Finds the best cycle up to `max_hops` starting from `start_token`
    pub fn find_best_cycle(
        graph: &MarketGraph,
        start_token: Pubkey,
        amount_in: u64,
        max_hops: u8,
    ) -> Option<SwapPath> {
        let mut best_path: Option<SwapPath> = None;
        let mut visited = Vec::new();
        let mut current_hops = Vec::new();
        
        Self::find_cycles_recursive(
            graph,
            start_token,
            start_token,
            amount_in,
            amount_in,
            max_hops,
            &mut visited,
            &mut current_hops,
            &mut best_path,
        );

        best_path
    }

    fn find_cycles_recursive(
        graph: &MarketGraph,
        current_token: Pubkey,
        start_token: Pubkey,
        current_amount: u64,
        initial_amount: u64,
        remaining_hops: u8,
        visited: &mut Vec<Pubkey>,
        current_path: &mut Vec<Edge>,
        best_path: &mut Option<SwapPath>,
    ) {
        if remaining_hops == 0 {
            return;
        }

        if let Some(edges) = graph.adj.get(&current_token) {
            for edge in edges {
                let amount_out = graph.get_amount_out(edge, current_amount);
                if amount_out == 0 { continue; }

                // Pruning: if current_amount is significantly lower than initial and few hops left
                // (Very aggressive pruning can be added here once we have slippage/fees accounted for)

                if edge.to_token == start_token {
                    let profit = amount_out as i64 - initial_amount as i64;
                    if profit > best_path.as_ref().map_or(0, |p| p.expected_profit) {
                        let mut final_path = current_path.clone();
                        final_path.push(edge.clone());
                        *best_path = Some(SwapPath {
                            hops: final_path,
                            expected_profit: profit,
                        });
                    }
                } else if !visited.contains(&edge.to_token) {
                    visited.push(edge.to_token);
                    current_path.push(edge.clone());
                    
                    Self::find_cycles_recursive(
                        graph,
                        edge.to_token,
                        start_token,
                        amount_out,
                        initial_amount,
                        remaining_hops - 1,
                        visited,
                        current_path,
                        best_path,
                    );
                    
                    current_path.pop();
                    visited.pop();
                }
            }
        }
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

        // Run search with 1 SOL input and 3 hops
        let path = ArbFinder::find_best_cycle(&graph, token_sol, 1_000_000, 3); // 0.001 SOL test
        
        // Should find a path
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.hops.len(), 3);
        assert!(p.expected_profit > 0);
    }

    #[test]
    fn test_find_4_hop_cycle() {
        let mut graph = MarketGraph::new();
        let t1 = Pubkey::new_unique();
        let t2 = Pubkey::new_unique();
        let t3 = Pubkey::new_unique();
        let t4 = Pubkey::new_unique();
        let p = mev_core::constants::RAYDIUM_V4_PROGRAM;

        // Path: T1 -> T2 -> T3 -> T4 -> T1
        // (Large reserves to avoid price impact in test)
        graph.update_edge(t1, t2, Pubkey::new_unique(), p, 1_000_000_000, 1_100_000_000, None, None);
        graph.update_edge(t2, t3, Pubkey::new_unique(), p, 1_000_000_000, 1_100_000_000, None, None);
        graph.update_edge(t3, t4, Pubkey::new_unique(), p, 1_000_000_000, 1_100_000_000, None, None);
        graph.update_edge(t4, t1, Pubkey::new_unique(), p, 1_000_000_000, 1_100_000_000, None, None);

        let path = ArbFinder::find_best_cycle(&graph, t1, 100, 4);
        assert!(path.is_some());
        assert_eq!(path.unwrap().hops.len(), 4);
    }
}
