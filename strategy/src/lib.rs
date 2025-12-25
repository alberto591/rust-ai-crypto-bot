pub mod ports;
pub mod adapters;
pub mod graph; // "The Brain" market graph
pub mod arb;   // "The Finder" search engine
pub mod analytics;
pub mod safety;

#[cfg(test)]
mod hft_tests;

#[cfg(test)]
mod profit_sanity_tests;



use mev_core::{PoolUpdate, ArbitrageOpportunity, SwapStep};
use std::sync::Arc;
use tracing::{info, debug, error, warn};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use parking_lot::RwLock;  // Faster than std::sync::Mutex
use smallvec::SmallVec;   // Stack-allocated vectors
use crate::analytics::volatility::VolatilityTracker;

use crate::ports::{AIModelPort, ExecutionPort, BundleSimulator, TelemetryPort};

pub struct StrategyEngine {
    arb_strategy: ArbitrageStrategy,
    executor: Option<Arc<dyn ExecutionPort>>,
    simulator: Option<Arc<dyn BundleSimulator>>,
    ai_model: Option<Arc<dyn AIModelPort>>,
    performance_tracker: Option<Arc<crate::analytics::performance::PerformanceTracker>>,
    safety_checker: Option<Arc<crate::safety::token_validator::TokenSafetyChecker>>,
    volatility_tracker: Arc<VolatilityTracker>,
    telemetry: Option<Arc<dyn TelemetryPort>>,  // NEW
    pub total_simulated_pnl: Arc<std::sync::atomic::AtomicU64>,
}

impl StrategyEngine {
    pub fn new(
        executor: Option<Arc<dyn ExecutionPort>>, 
        simulator: Option<Arc<dyn BundleSimulator>>,
        ai_model: Option<Arc<dyn AIModelPort>>,
        performance_tracker: Option<Arc<crate::analytics::performance::PerformanceTracker>>,
        safety_checker: Option<Arc<crate::safety::token_validator::TokenSafetyChecker>>,
        telemetry: Option<Arc<dyn TelemetryPort>>,
    ) -> Self {
        let volatility_tracker = Arc::new(VolatilityTracker::new());
        Self {
            arb_strategy: ArbitrageStrategy::new(Arc::clone(&volatility_tracker)),
            executor,
            simulator,
            ai_model,
            performance_tracker,
            safety_checker,
            volatility_tracker,
            telemetry,
            total_simulated_pnl: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn process_event(
        &self, 
        update: PoolUpdate, 
        initial_amount: u64,
        jito_tip_lamports: u64,
        jito_tip_percentage: f64,
        max_jito_tip_lamports: u64,
        max_slippage_bps: u16,
        volatility_sensitivity: f64,
        max_slippage_ceiling: u16,
    ) -> anyhow::Result<Option<ArbitrageOpportunity>> {
        // ... (Safety gates etc) ...
        // ... (Update Graph & Find Cycle) ...

        // 🛡️ SAFETY GATES (Institutional Grade)
        const MAX_TRADE_SIZE: u64 = 1_000_000_000; // 1.0 SOL (Panic Limit)
        const MIN_PROFIT_THRESHOLD: u64 = 15_000;  // Lowered to 15k to catch smaller opportunities
        
        // Check 1: Is the bet too big?
        if initial_amount > MAX_TRADE_SIZE {
            error!("⛔ SAFETY TRIGGER: Trade size {} exceeds limit!", initial_amount);
            return Ok(None);
        }

        // 1. Update Graph & Find Cycle
        let opportunity = match self.arb_strategy.process_update(update, initial_amount) {
            Some(opp) => opp,
            None => return Ok(None),
        };

        // 2. Dynamic Tip Calculation
        let profit = opportunity.expected_profit_lamports;
        
        // 2.1 Profit Sanity Check: Reject unrealistic profits
        // If profit > 10% of input, likely bad data (stale prices, flash crash, or bug)
        let max_reasonable_profit = initial_amount / 10;  // 10% of input
        if profit > max_reasonable_profit {
            warn!("⛔ SANITY CHECK: Profit {} lamports ({}%) exceeds reasonable threshold {}. Likely stale data or calculation error. Rejecting opportunity.",
                profit, 
                (profit * 100) / initial_amount,
                max_reasonable_profit
            );
            
            if let Some(ref tel) = self.telemetry {
                tel.log_profit_sanity_rejection();
            }
            return Ok(None);
        }
        
        let mut tip_lamports = (profit as f64 * jito_tip_percentage) as u64;
        
        // Apply floor and ceiling
        tip_lamports = tip_lamports.max(jito_tip_lamports); // Floor at base tip
        tip_lamports = tip_lamports.min(max_jito_tip_lamports); // Ceiling at max tip
        
        // Final sanity check: Tip must be less than profit
        if tip_lamports >= profit {
            warn!("⛔ SAFETY: Calculated tip {} is >= profit {}. Aborting trade.", tip_lamports, profit);
            return Ok(None);
        }

        // Check 2: Is the profit worth the gas? (After tip)
        let net_profit = profit.saturating_sub(tip_lamports);
        if net_profit < MIN_PROFIT_THRESHOLD {
            debug!("⛔ SAFETY TRIGGER: Net profit {} is too small.", net_profit);
            return Ok(None);
        }

        info!("💡 Profitable path found: {} lamports expected (Tip: {}).", profit, tip_lamports);
        println!("🚀 ARB_FOUND: {} hops, profit: {} lamports", opportunity.steps.len(), opportunity.expected_profit_lamports);

            // 2. AI validation layer
            let ai_confidence = if let Some(model) = &self.ai_model {
                model.predict_confidence(&opportunity).unwrap_or(0.0)
            } else {
                1.0 // Heuristic mode: assumes perfect confidence
            }; 
            
            if ai_confidence < 0.8 {
                 debug!("⚠️ Opportunity rejected by AI Model (Confidence: {:.2})", ai_confidence);
                 return Ok(None);
            }

            info!("🚀 AI Approved: High confidence ({:.2}). Triggering execution pipeline...", ai_confidence);
            
            // 2.5 Safety Filter (Rug Shield)
            if let Some(checker) = &self.safety_checker {
                // Check all output mints in the path (excluding the start/end which is usually SOL/USDC)
                for step in &opportunity.steps {
                    if !checker.is_safe_to_trade(&step.output_mint, &step.pool).await {
                        warn!("⛔ SAFETY: Token {} in pool {} failed safety check. Aborting trade.", step.output_mint, step.pool);
                        if let Some(ref tel) = self.telemetry {
                            tel.log_safety_rejection();
                        }
                        return Ok(None);
                    }
                }
            }

            // 3. Infrastructure interaction via Ports
            if let Some(executor) = &self.executor {
                // Dynamic Slippage Calculation
                let mut effective_slippage = max_slippage_bps;
                
                // Calculate max volatility among pools in the cycle
                let mut max_vol = 0.0_f64;
                for step in &opportunity.steps {
                    max_vol = max_vol.max(self.volatility_tracker.get_volatility_factor(step.pool));
                }
                
                if max_vol > 0.0 {
                    let vol_adjustment = (1.0 + max_vol * volatility_sensitivity) as f64;
                    effective_slippage = (max_slippage_bps as f64 * vol_adjustment) as u16;
                    effective_slippage = effective_slippage.min(max_slippage_ceiling);
                    
                    if effective_slippage > max_slippage_bps {
                        info!("📈 Volatility Detected ({:.4}). Adjusting slippage: {}bps -> {}bps", max_vol, max_slippage_bps, effective_slippage);
                    }
                }

                // Optional Simulation
                if let Some(simulator) = &self.simulator {
                    let instructions = executor.build_bundle_instructions(
                        opportunity.clone(), 
                        tip_lamports, 
                        effective_slippage
                    ).await?;
                    match simulator.simulate_bundle(&instructions, executor.pubkey()).await {
                        Ok(units) => info!("✅ Simulation confirmed: {} units.", units),
                        Err(e) => {
                            warn!("❌ Simulation fail: {}. Dropping trade.", e);
                            return Ok(None);
                        }
                    }
                }

                // 4. Track stats
                self.total_simulated_pnl.fetch_add(opportunity.expected_profit_lamports, std::sync::atomic::Ordering::SeqCst);

                // 4.5 Log to Performance Tracker (Non-blocking)
                if let Some(tracker) = &self.performance_tracker {
                    let token_label = format!("{:?}", opportunity.steps.last().map(|s| s.output_mint));
                    tracker.log_trade(&token_label, opportunity.expected_profit_lamports as i64, "Live").await;
                }

                // 5. Atomic Execution
                match executor.build_and_send_bundle(
                    opportunity.clone(), 
                    solana_sdk::hash::Hash::default(), 
                    tip_lamports,
                    effective_slippage
                ).await {
                    Ok(bundle_id) => {
                        info!("🔥 BUNDLE DISPATCHED: {}", bundle_id);
                        return Ok(Some(opportunity));
                    },
                    Err(e) => {
                        error!("💥 Execution panic: {}", e);
                        return Ok(None);
                    }
                }
            } else {
                return Ok(Some(opportunity));
            }
        }
    }

pub struct ArbitrageStrategy {
    graph: RwLock<DiGraph<Pubkey, Vec<PoolUpdate>>>,  // HFT: RwLock for concurrent reads, Vec for multi-pool support
    nodes: RwLock<HashMap<Pubkey, NodeIndex>>,   // Read-heavy workload
    volatility_tracker: Arc<VolatilityTracker>,
}

impl Default for ArbitrageStrategy {
    fn default() -> Self {
        Self::new(Arc::new(VolatilityTracker::new()))
    }
}

impl ArbitrageStrategy {
    pub fn new(volatility_tracker: Arc<VolatilityTracker>) -> Self {
        Self {
            graph: RwLock::new(DiGraph::new()),
            nodes: RwLock::new(HashMap::new()),
            volatility_tracker,
        }
    }

    pub fn process_update(&self, update: PoolUpdate, initial_amount: u64) -> Option<ArbitrageOpportunity> {
        // HFT OPTIMIZATION: Minimize write-lock duration
        
        // 1. Fast path: Try read-only lookup first
        let (node_a, node_b) = {
            let nodes_read = self.nodes.read();
            (nodes_read.get(&update.mint_a).copied(), nodes_read.get(&update.mint_b).copied())
        };
        
        // 2. If nodes exist, upgrade to write for edge update
        let (node_a, node_b) = match (node_a, node_b) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                // Write path: Need to create new nodes
                let mut graph = self.graph.write();
                let mut nodes = self.nodes.write();
                
                let a = *nodes.entry(update.mint_a).or_insert_with(|| graph.add_node(update.mint_a));
                let b = *nodes.entry(update.mint_b).or_insert_with(|| graph.add_node(update.mint_b));
                
                tracing::info!("🧠 Graph Updated: {} Nodes, {} Edges", graph.node_count(), graph.edge_count());
                (a, b)
            }
        };

        // 3. Update the market graph
        {
            let mut graph = self.graph.write();
            let update_edge = |graph: &mut DiGraph<Pubkey, Vec<PoolUpdate>>, from, to, data: PoolUpdate| {
                if let Some(edge_idx) = graph.find_edge(from, to) {
                    let pools = &mut graph[edge_idx];
                    // Find existing pool with same address and update it, or add new pool
                    if let Some(pool) = pools.iter_mut().find(|p| p.pool_address == data.pool_address) {
                        let pool_addr = data.pool_address;
                        *pool = data;  // Update existing pool
                        tracing::debug!("Updated existing pool {} in edge", pool_addr);
                    } else {
                        let pool_addr = data.pool_address;
                        let new_len = pools.len() + 1;
                        pools.push(data);  // Add new pool for cross-DEX
                        tracing::info!("🔗 Added new pool {} to edge (total: {})", pool_addr, new_len);
                    }
                } else {
                    let pool_addr = data.pool_address;  // Copy before move
                    graph.add_edge(from, to, vec![data]);
                    tracing::debug!("Created new edge with pool {}", pool_addr);
                }
            };
            update_edge(&mut graph, node_a, node_b, update.clone());
            update_edge(&mut graph, node_b, node_a, update.clone());
        }

        // 3.5 Update Volatility Tracker
        let price = if update.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM {
            let sqrt_p = update.price_sqrt.unwrap_or(0) as f64 / (1u128 << 64) as f64;
            sqrt_p * sqrt_p
        } else {
            if update.reserve_a > 0 {
                update.reserve_b as f64 / update.reserve_a as f64
            } else {
                0.0
            }
        };
        if price > 0.0 {
            self.volatility_tracker.add_sample(update.pool_address, price);
        }

        // 4. Search for cycles (read-lock only)
        let graph = self.graph.read();
        let max_hops = 5;
        let mut best_opp: Option<ArbitrageOpportunity> = None;
        let mut visited: SmallVec<[NodeIndex; 8]> = SmallVec::new();  // Stack-allocated for common case
        visited.push(node_a);
        
        tracing::debug!("🔍 Searching for cycles from node {:?} (mint: {})", node_a, update.mint_a);

        self.find_cycles_recursive(&graph, node_a, node_a, initial_amount, initial_amount, &mut visited, &mut SmallVec::new(), &mut best_opp, max_hops);
        
        if best_opp.is_some() {
            tracing::info!("✅ Cycle found!");
        }
        
        best_opp
    }

    fn find_cycles_recursive(
        &self,
        graph: &DiGraph<Pubkey, Vec<PoolUpdate>>,
        current_node: NodeIndex,
        start_node: NodeIndex,
        current_amount: u64,
        initial_amount: u64,
        visited: &mut SmallVec<[NodeIndex; 8]>,      // HFT: Stack-allocated
        current_steps: &mut SmallVec<[SwapStep; 8]>, // HFT: Stack-allocated
        best_opp: &mut Option<ArbitrageOpportunity>,
        remaining_hops: u8,
    ) {
        if remaining_hops == 0 { return; }

        let current_mint = graph[current_node];
        let _start_mint = graph[start_node];
        
        let edge_count = graph.edges(current_node).count();
        tracing::debug!(
            "  [Hop {}] At node {:?} (mint: {}), amount: {}, edges: {}",
            5 - remaining_hops,
            current_node,
            current_mint,
            current_amount,
            edge_count
        );

        // Track metrics for 5-hop features
        let mut total_fees_bps: u16 = 0;
        let mut max_price_impact_bps: u16 = 0;
        let mut min_liquidity: u128 = u128::MAX;

        for edge in graph.edges(current_node) {
            let pools = edge.weight();  // Now Vec<PoolUpdate>
            let next_node = edge.target();
            let next_mint = graph[next_node];
            
            tracing::debug!(
                "    → Edge to {:?} (mint: {}), {} pool(s) available",
                next_node,
                next_mint,
                pools.len()
            );
            
            // Try each pool in this edge (enables cross-DEX arbitrage)
            for pool in pools {
                tracing::debug!(
                    "      Pool: {}, program: {}",
                    pool.pool_address,
                    if pool.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM { "Orca" } else { "Raydium" }
                );

            // 1. Calculate reserves and amount out based on DEX type
            let (res_in, amount_out) = if pool.program_id == mev_core::constants::ORCA_WHIRLPOOL_PROGRAM {
                let price_sqrt = pool.price_sqrt.unwrap_or(0);
                let liquidity = pool.liquidity.unwrap_or(0);
                
                // Virtual reserve approximation for impact calculation
                let sqrt_p = price_sqrt as f64 / (1u128 << 64) as f64;
                let a_to_b = pool.mint_a == current_mint;
                let v_res_in = if a_to_b {
                    (liquidity as f64 / sqrt_p) as u64
                } else {
                    (liquidity as f64 * sqrt_p) as u64
                };

                (v_res_in, mev_core::math::get_amount_out_clmm(current_amount, price_sqrt, liquidity, pool.fee_bps, a_to_b))
            } else {
                let (r_in, r_out) = if pool.mint_a == current_mint {
                    (pool.reserve_a as u64, pool.reserve_b as u64)
                } else {
                    (pool.reserve_b as u64, pool.reserve_a as u64)
                };
                (r_in, mev_core::math::get_amount_out_cpmm(current_amount, r_in, r_out, pool.fee_bps))
            };

            tracing::debug!("      Calculated amount_out: {}", amount_out);

            if amount_out == 0 { 
                tracing::debug!("      ✗ Skipped: amount_out = 0");
                continue; 
            }

            // 1.5 Price Impact Check (Phase 6C)
            let impact = mev_core::math::calculate_price_impact(current_amount, res_in);
            if (impact * 10000.0) as u16 > 100 { // 1% Max Impact
                debug!("Skipping path due to high price impact: {:.2}%", impact * 100.0);
                continue;
            }

            // Update metrics
            total_fees_bps += pool.fee_bps;
            let current_impact_bps = (impact * 10000.0) as u16;
            max_price_impact_bps = max_price_impact_bps.max(current_impact_bps);
            min_liquidity = min_liquidity.min(res_in as u128);

            // 2. Prepare swap step
            let step = SwapStep {
                pool: pool.pool_address,
                program_id: pool.program_id,
                input_mint: current_mint,
                output_mint: next_mint,
                expected_output: amount_out,
            };

            // 3. Cycle detected?
            if next_node == start_node {
                tracing::info!(
                    "      🔄 CYCLE DETECTED! Start amount: {}, End amount: {}, Profit: {}",
                    initial_amount,
                    amount_out,
                    if amount_out > initial_amount { amount_out - initial_amount } else { 0 }
                );
                
                if amount_out > initial_amount { // Use provided initial amount
                    let profit = amount_out - initial_amount;
                    let mut steps = current_steps.clone();
                    steps.push(step);
                    
                    tracing::info!("      ✅ PROFITABLE CYCLE! Profit: {} lamports", profit);
                    
                    if best_opp.as_ref().is_none_or(|o| profit > o.expected_profit_lamports) {
                        *best_opp = Some(ArbitrageOpportunity {
                            steps: steps.to_vec(),  // Convert SmallVec to Vec for API
                            expected_profit_lamports: profit,
                            input_amount: initial_amount,
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
                    initial_amount,
                    visited,
                    current_steps,
                    best_opp,
                    remaining_hops - 1,
                );
                current_steps.pop();
                visited.pop();
            }
            }  // End of: for pool in pools
        }  // End of: for edge in graph.edges(current_node)
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
            price_sqrt: None,
            liquidity: None,
            fee_bps: 0,
            timestamp: 0,
        }
    }

    fn mock_orca_pool(addr: &str, mint_a: &str, mint_b: &str, price_sqrt: u128, liquidity: u128) -> PoolUpdate {
        PoolUpdate {
            pool_address: addr.parse().unwrap(),
            program_id: mev_core::constants::ORCA_WHIRLPOOL_PROGRAM,
            mint_a: mint_a.parse().unwrap(),
            mint_b: mint_b.parse().unwrap(),
            reserve_a: 0,
            reserve_b: 0,
            price_sqrt: Some(price_sqrt),
            liquidity: Some(liquidity),
            fee_bps: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn test_multi_hop_cycle_detection() {
        let strategy = ArbitrageStrategy::new(Arc::new(VolatilityTracker::new()));
        
        let mint_sol = "So11111111111111111111111111111111111111112";
        let mint_usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let mint_ray = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
        let mint_bonk = "DezXv2uHrqGqS78vJDRRua6nJFr1rU7mFT9S8NoA1Fp";

        // Create a 4-hop profitable cycle: SOL -> USDC -> BONK -> RAY -> SOL
        // All pools must be deep enough for a 1 SOL (1B lamport) trade
        // SOL/USDC: 1 SOL = 100 USDC (Reserves: 100,000 SOL / 10,000,000 USDC)
        strategy.process_update(mock_pool("58oQChGsNrtmhaJSRph38tB3BwpL66F42FMa86Fv3Gry", mint_sol, mint_usdc, 100_000_000_000_000, 10_000_000_000_000_000), 1_000_000_000);
        // USDC/BONK: 100 USDC = 100M BONK (Reserves: 10,000,000 USDC / 10,000,000,000,000 BONK)
        strategy.process_update(mock_pool("AVs91fXYvQJdufSs6S6S8kSEbd67QpUtyUfV8vUjJsc", mint_usdc, mint_bonk, 10_000_000_000_000_000, 10_000_000_000_000_000_000), 1_000_000_000);
        // BONK/RAY: 100M BONK = 50 RAY (Reserves: 10,000,000,000,000 BONK / 5,000_000_000_000 lamports)
        strategy.process_update(mock_pool("DZ6ayPbaB9p8Kx7tH5rTMGidMjgjM8HhnRizAnV8hX5P", mint_bonk, mint_ray, 10_000_000_000_000_000_000, 5_000_000_000_000_000_000), 1_000_000_000);
        // RAY/SOL: 50 RAY = 1.1 SOL (Reserves: 5,000_000_000_000 lamports / 110,000_000_000 lamports)
        let final_update = mock_pool("7XawhbbxtsRcQA8KTkHT9f9nc6d69UeMvdxS1ioL69hY", mint_ray, mint_sol, 5_000_000_000_000_000_000, 110_000_000_000_000_000_000);
        
        let opp = strategy.process_update(final_update, 1_000_000_000).expect("Should find cycle");
        
        assert_eq!(opp.steps.len(), 4);
        assert!(opp.expected_profit_lamports > 0);
        // The cycle starts from the first token of the update that triggered it (RAY)
        assert_eq!(opp.steps[0].input_mint, mint_ray.parse::<Pubkey>().unwrap());
        assert_eq!(opp.steps[3].output_mint, mint_ray.parse::<Pubkey>().unwrap());
    }

    #[test]
    fn test_slippage_rejection() {
        let strategy = ArbitrageStrategy::new(Arc::new(VolatilityTracker::new()));
        let mint_sol = "So11111111111111111111111111111111111111112";
        let mint_usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let mint_ray = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";

        // Create a cycle but with high price impact on one leg
        // SOL/USDC (Deep)
        strategy.process_update(mock_pool("58oQChGsNrtmhaJSRph38tB3BwpL66F42FMa86Fv3Gry", mint_sol, mint_usdc, 1_000_000_000_000, 100_000_000_000_000), 1_000_000_000);
        // USDC/RAY (Deep)
        strategy.process_update(mock_pool("AVs91fXYvQJdufSs6S6S8kSEbd67QpUtyUfV8vUjJsc", mint_usdc, mint_ray, 100_000_000_000_000, 1_000_000_000_000_000), 1_000_000_000);
        // RAY/SOL (SHALLOW POOL: Only 1B lamports, trading 1B. Impact = 50%)
        let shallow_update = mock_pool("DZ6ayPbaB9p8Kx7tH5rTMGidMjgjM8HhnRizAnV8hX5P", mint_ray, mint_sol, 1_000_000_000, 1_000_000_000);
        
        let opp = strategy.process_update(shallow_update, 1_000_000_000);
        
        // Should be None because price impact > 1%
        assert!(opp.is_none());
    }

    #[test]
    fn test_0_1_sol_triangular_arb() {
        let strategy = ArbitrageStrategy::new(Arc::new(VolatilityTracker::new()));
        let initial_amount = 100_000_000; // 0.1 SOL
        
        let mint_sol = Pubkey::new_unique();
        let mint_usdc = Pubkey::new_unique();
        let mint_usdt = Pubkey::new_unique();

        // 1. SOL/USDC: 1 SOL = 200 USDC (Deep pool)
        strategy.process_update(mock_pool(&Pubkey::new_unique().to_string(), &mint_sol.to_string(), &mint_usdc.to_string(), 100_000_000_000, 20_000_000_000_000), initial_amount);
        // 2. USDC/USDT: 1 USDC = 1 USDT (Deep pool)
        strategy.process_update(mock_pool(&Pubkey::new_unique().to_string(), &mint_usdc.to_string(), &mint_usdt.to_string(), 100_000_000_000_000, 100_000_000_000_000), initial_amount);
        // 3. USDT/SOL: 1 USDT = 0.01 SOL (1 SOL = 100 USDT). 
        // Deep reserves to keep price impact < 1% for 20B USDT input.
        let final_update = mock_pool(&Pubkey::new_unique().to_string(), &mint_usdt.to_string(), &mint_sol.to_string(), 2_000_000_000_000, 20_000_000_000);
        
        let opp = strategy.process_update(final_update, initial_amount).expect("Should find cycle");

        
        assert_eq!(opp.steps.len(), 3);
        assert_eq!(opp.input_amount, initial_amount);
        assert!(opp.expected_profit_lamports > initial_amount / 2); // Should be roughly 0.1 SOL profit
    }

    #[test]
    fn test_cross_dex_arbitrage() {
        let strategy = ArbitrageStrategy::new(Arc::new(VolatilityTracker::new()));
        let initial_amount = 1_000_000_000; // 1 SOL
        
        let mint_sol = "So11111111111111111111111111111111111111112";
        let mint_usdc = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

        // 1. Raydium: SOL -> USDC (1 SOL = 100 USDC)
        // Deep reserves: 10B SOL / 1T USDC
        strategy.process_update(mock_pool("58oQChGsNrtmhaJSRph38tB3BwpL66F42FMa86Fv3Gry", mint_sol, mint_usdc, 10_000_000_000, 1_000_000_000_000), initial_amount);
        
        // 2. Orca: USDC -> SOL (1 USDC = 0.011 SOL -> 100 USDC = 1.1 SOL)
        let price = 0.011;
        let sqrt_p = (price as f64).sqrt() * (1u128 << 64) as f64;
        let orca_update = mock_orca_pool("whirLbMiqkh6thXv7uBToywS9Bn1McGQ669YUsbAHQi", mint_usdc, mint_sol, sqrt_p as u128, 1_000_000_000_000);
        
        let opp = strategy.process_update(orca_update, initial_amount).expect("Should find cross-dex cycle");
        
        assert_eq!(opp.steps.len(), 2);
        assert!(opp.expected_profit_lamports > 0);
        
        // Cycle starts from USDC (triggering update mint_a)
        // Step 0: USDC -> SOL (Orca)
        // Step 1: SOL -> USDC (Raydium)
        assert_eq!(opp.steps[0].program_id, mev_core::constants::ORCA_WHIRLPOOL_PROGRAM);
        assert_eq!(opp.steps[1].program_id, mev_core::constants::RAYDIUM_V4_PROGRAM);
    }
}
