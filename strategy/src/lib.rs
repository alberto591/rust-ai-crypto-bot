pub mod ports;
pub mod adapters;
pub mod graph; // "The Brain" market graph


use mev_core::{PoolUpdate, ArbitrageOpportunity, SwapStep, math::get_amount_out_cpmm};
use std::sync::{Mutex, Arc};
use tracing::{info, debug, error, warn};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use anyhow::Result;

use crate::ports::{AIModelPort, ExecutionPort, BundleSimulator};
use solana_sdk::instruction::Instruction;

// BundleSimulator trait is imported from ports


pub struct StrategyEngine {
    arb_strategy: ArbitrageStrategy,
    executor: Option<Arc<dyn ExecutionPort>>,
    simulator: Option<Arc<dyn BundleSimulator>>,
    ai_model: Option<Arc<dyn AIModelPort>>,
    pub total_simulated_pnl: Arc<std::sync::atomic::AtomicU64>,
}

impl StrategyEngine {
    pub fn new(
        executor: Option<Arc<dyn ExecutionPort>>, 
        simulator: Option<Arc<dyn BundleSimulator>>,
        ai_model: Option<Arc<dyn AIModelPort>>
    ) -> Self {
        Self {
            arb_strategy: ArbitrageStrategy::new(),
            executor,
            simulator,
            ai_model,
            total_simulated_pnl: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn process_event(&self, update: PoolUpdate) -> Result<Option<ArbitrageOpportunity>> {
        // 1. Hard Logic: Multi-DEX Triangular Arbitrage Math
        if let Some(opportunity) = self.arb_strategy.process_update(update.clone()) {
            info!("Hueristic check: profitable path found ({} lamports expected).", opportunity.expected_profit_lamports);

            // 2. AI Logic: Risk/Confidence Assessment
            let ai_confidence = if let Some(model) = &self.ai_model {
                model.predict_confidence(&opportunity).unwrap_or(1.0)
            } else {
                1.0 // Heuristic mode: always confident
            }; 
            
            if ai_confidence > 0.7 {
                info!("Decision: Strong Buy (Confidence: {:.2}). Simulating bundle...", ai_confidence);
                
                // 3. Simulation (Safety & Optimization)
                if let Some(executor) = &self.executor {
                    let tip_lamports = 100_000;
                    
                    // Build instructions first for simulation
                    let instructions = match executor.build_bundle_instructions(opportunity.clone(), tip_lamports).await {
                        Ok(ins) => ins,
                        Err(e) => {
                            error!("Failed to build bundle instructions: {}", e);
                            return Ok(None);
                        }
                    };

                    if let Some(simulator) = &self.simulator {
                        match simulator.simulate_bundle(&instructions, &executor.pubkey()).await {
                            Ok(units) => info!("Simulation SUCCEEDED: {} units consumed.", units),
                            Err(e) => {
                                warn!("Simulation FAILED: {}. DROPPING trade.", e);
                                return Ok(None);
                            }
                        }
                    }

                    // 4. Track simulated PnL
                    self.total_simulated_pnl.fetch_add(opportunity.expected_profit_lamports, std::sync::atomic::Ordering::SeqCst);

                    // 5. Execution (Integration with Phase 2)
                    let recent_blockhash = solana_sdk::hash::Hash::default(); // In main loop, this is updated
                    
                    match executor.build_and_send_bundle(opportunity.clone(), recent_blockhash, tip_lamports).await {
                        Ok(id) => {
                            info!("Bundle executed: {}", id);
                            return Ok(Some(opportunity));
                        },
                        Err(e) => {
                            error!("Execution failed: {}", e);
                            return Ok(None);
                        },
                    }
                } else {
                    warn!("Opportunity found but Execution is DISABLED (No Jito Client).");
                }
            } else {
                debug!("Opportunity rejected by AI Brain (Confidence: {:.2})", ai_confidence);
            }
        }

        Ok(None)
    }


}

pub struct ArbitrageStrategy {
    graph: Mutex<DiGraph<Pubkey, PoolUpdate>>,
    nodes: Mutex<HashMap<Pubkey, NodeIndex>>,
}

impl ArbitrageStrategy {
    pub fn new() -> Self {
        Self {
            graph: Mutex::new(DiGraph::new()),
            nodes: Mutex::new(HashMap::new()),
        }
    }

    pub fn process_update(&self, update: PoolUpdate) -> Option<ArbitrageOpportunity> {
        let mut graph = self.graph.lock().unwrap();
        let mut nodes = self.nodes.lock().unwrap();

        let node_a = *nodes.entry(update.mint_a).or_insert_with(|| graph.add_node(update.mint_a));
        let node_b = *nodes.entry(update.mint_b).or_insert_with(|| graph.add_node(update.mint_b));

        // Update or add edges in both directions (AMM pool)
        let mut update_edge = |from, to, data: PoolUpdate| {
            if let Some(edge) = graph.find_edge(from, to) {
                graph[edge] = data;
            } else {
                graph.add_edge(from, to, data);
            }
        };

        update_edge(node_a, node_b, update.clone());
        update_edge(node_b, node_a, update.clone());

        // DFS for cycles starting and ending at node_a
        let max_hops = 5;
        let initial_amount = 1_000_000_000u64; // 1 SOL
        let mut best_opp: Option<ArbitrageOpportunity> = None;
        let mut visited = vec![node_a];

        self.find_cycles_recursive(&graph, node_a, node_a, initial_amount, &mut visited, &mut vec![], &mut best_opp, max_hops);
        
        best_opp
    }

    fn find_cycles_recursive(
        &self,
        graph: &DiGraph<Pubkey, PoolUpdate>,
        current_node: NodeIndex,
        start_node: NodeIndex,
        current_amount: u64,
        visited: &mut Vec<NodeIndex>,
        current_steps: &mut Vec<SwapStep>,
        best_opp: &mut Option<ArbitrageOpportunity>,
        remaining_hops: u8,
    ) {
        if remaining_hops == 0 { return; }

        let current_mint = graph[current_node];
        let _start_mint = graph[start_node];

        // Track metrics for 5-hop features
        let mut total_fees_bps: u16 = 0;
        let mut max_price_impact_bps: u16 = 0;
        let mut min_liquidity: u128 = u128::MAX;

        for edge in graph.edges(current_node) {
            let pool = edge.weight();
            let next_node = edge.target();
            let next_mint = graph[next_node];

            // 1. Calculate amount out
            let (res_in, res_out) = if pool.mint_a == current_mint {
                (pool.reserve_a as u64, pool.reserve_b as u64)
            } else {
                (pool.reserve_b as u64, pool.reserve_a as u64)
            };
            
            let amount_out = get_amount_out_cpmm(current_amount, res_in, res_out, pool.fee_bps);
            if amount_out == 0 { continue; }

            // 1.5 Price Impact Check (Phase 6C)
            let price_impact = mev_core::math::calculate_price_impact(current_amount, res_in);
            let price_impact_bps = (price_impact * 10000.0) as u16;
            
            if price_impact > 0.01 { // Reject trades with > 1% price impact
                debug!("Skipping path due to high price impact: {:.2}%", price_impact * 100.0);
                continue;
            }

            // Update metrics
            total_fees_bps += pool.fee_bps;
            max_price_impact_bps = max_price_impact_bps.max(price_impact_bps);
            min_liquidity = min_liquidity.min(pool.reserve_a.min(pool.reserve_b));

            // 2. Prepare swap step
            let step = SwapStep {
                pool: pool.pool_address,
                program_id: pool.program_id,
                input_mint: current_mint,
                output_mint: next_mint,
            };

            // 3. Cycle detected?
            if next_node == start_node {
                if amount_out > 1_000_000_000 { // Start amount
                    let profit = amount_out - 1_000_000_000;
                    let mut steps = current_steps.clone();
                    steps.push(step);
                    
                    if best_opp.as_ref().map_or(true, |o| profit > o.expected_profit_lamports) {
                        *best_opp = Some(ArbitrageOpportunity {
                            steps,
                            expected_profit_lamports: profit,
                            input_amount: 1_000_000_000,
                            total_fees_bps,
                            max_price_impact_bps,
                            min_liquidity,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        });
                    }
                }
                continue;
            }

            // 4. Recurse if not visited in this path
            if !visited.contains(&next_node) {
                visited.push(next_node);
                current_steps.push(step);
                
                self.find_cycles_recursive(
                    graph,
                    next_node,
                    start_node,
                    amount_out,
                    visited,
                    current_steps,
                    best_opp,
                    remaining_hops - 1,
                );
                current_steps.pop();
                visited.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mev_core::constants::RAYDIUM_V4_PROGRAM;
    use solana_sdk::pubkey::Pubkey;

    fn mock_pool(addr: &str, mint_a: &str, mint_b: &str, res_a: u128, res_b: u128) -> PoolUpdate {
        PoolUpdate {
            pool_address: addr.parse().unwrap(),
            program_id: RAYDIUM_V4_PROGRAM,
            mint_a: mint_a.parse().unwrap(),
            mint_b: mint_b.parse().unwrap(),
            reserve_a: res_a,
            reserve_b: res_b,
            fee_bps: 0, // 0 fee for simple math in tests
            timestamp: 0,
        }
    }

    #[test]
    fn test_multi_hop_cycle_detection() {
        let strategy = ArbitrageStrategy::new();
        
        let mint_sol = "So11111111111111111111111111111111111111112";
        let mint_usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let mint_ray = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
        let mint_bonk = "DezXv2uHrqGqS78vJDRRua6nJFr1rU7mFT9S8NoA1Fp";

        // Create a 4-hop profitable cycle: SOL -> USDC -> BONK -> RAY -> SOL
        // All pools must be deep enough for a 1 SOL (1B lamport) trade
        // SOL/USDC: 1 SOL = 100 USDC (Reserves: 100,000 SOL / 10,000,000 USDC)
        strategy.process_update(mock_pool("58oQChGsNrtmhaJSRph38tB3BwpL66F42FMa86Fv3Gry", mint_sol, mint_usdc, 100_000_000_000_000, 10_000_000_000_000_000));
        // USDC/BONK: 100 USDC = 100M BONK (Reserves: 10,000,000 USDC / 10,000,000,000,000 BONK)
        strategy.process_update(mock_pool("AVs91fXYvQJdufSs6S6S8kSEbd67QpUtyUfV8vUjJsc", mint_usdc, mint_bonk, 10_000_000_000_000_000, 10_000_000_000_000_000_000));
        // BONK/RAY: 100M BONK = 50 RAY (Reserves: 10,000,000,000,000 BONK / 5,000,000,000,000 lamports)
        strategy.process_update(mock_pool("DZ6ayPbaB9p8Kx7tH5rTMGidMjgjM8HhnRizAnV8hX5P", mint_bonk, mint_ray, 10_000_000_000_000_000_000, 5_000_000_000_000_000_000));
        // RAY/SOL: 50 RAY = 1.1 SOL (Reserves: 5,000,000,000,000 lamports / 110,000,000,000 lamports)
        let final_update = mock_pool("7XawhbbxtsRcQA8KTkHT9f9nc6d69UeMvdxS1ioL69hY", mint_ray, mint_sol, 5_000_000_000_000_000_000, 110_000_000_000_000_000_000);
        
        let opp = strategy.process_update(final_update).expect("Should find cycle");
        
        assert_eq!(opp.steps.len(), 4);
        assert!(opp.expected_profit_lamports > 0);
        // The cycle starts from the first token of the update that triggered it (RAY)
        assert_eq!(opp.steps[0].input_mint, mint_ray.parse::<Pubkey>().unwrap());
        assert_eq!(opp.steps[3].output_mint, mint_ray.parse::<Pubkey>().unwrap());
    }

    #[test]
    fn test_slippage_rejection() {
        let strategy = ArbitrageStrategy::new();
        let mint_sol = "So11111111111111111111111111111111111111112";
        let mint_usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let mint_ray = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";

        // Create a cycle but with high price impact on one leg
        // SOL/USDC (Deep)
        strategy.process_update(mock_pool("58oQChGsNrtmhaJSRph38tB3BwpL66F42FMa86Fv3Gry", mint_sol, mint_usdc, 1_000_000_000_000, 100_000_000_000_000));
        // USDC/RAY (Deep)
        strategy.process_update(mock_pool("AVs91fXYvQJdufSs6S6S8kSEbd67QpUtyUfV8vUjJsc", mint_usdc, mint_ray, 100_000_000_000_000, 1_000_000_000_000_000));
        // RAY/SOL (SHALLOW POOL: Only 1B lamports, trading 1B. Impact = 50%)
        let shallow_update = mock_pool("DZ6ayPbaB9p8Kx7tH5rTMGidMjgjM8HhnRizAnV8hX5P", mint_ray, mint_sol, 1_000_000_000, 1_000_000_000);
        
        let opp = strategy.process_update(shallow_update);
        
        // Should be None because price impact > 1%
        assert!(opp.is_none());
    }
}
