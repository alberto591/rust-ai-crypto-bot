mod listener;
mod config;
mod recorder;
mod simulation;
mod tui; 

use std::sync::{Arc, Mutex};
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tracing::{info, debug, error, warn, Level};
use tracing_subscriber::{FmtSubscriber, prelude::*, Layer, EnvFilter};
use mev_core::PoolUpdate;
use strategy::StrategyEngine;
use executor::JitoClient;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use listener::start_listener;
use config::BotConfig;
use recorder::AsyncCsvWriter;
use tui::{TuiApp, AppState};

// Custom Layer to capture logs for TUI
struct TuiLogLayer {
    state: Arc<Mutex<AppState>>,
}

impl<S> Layer<S> for TuiLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        struct MessageVisitor {
            msg: String,
        }
        impl tracing::field::Visit for MessageVisitor {
             fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                 if field.name() == "message" {
                     self.msg = format!("{:?}", value);
                 }
             }
             fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                 if field.name() == "message" {
                     self.msg = value.to_string();
                 }
             }
        }
        
        let mut visitor = MessageVisitor { msg: String::new() };
        event.record(&mut visitor);

        if !visitor.msg.is_empty() {
             // Strip quotes from Debug format if present
             let clean_msg = visitor.msg.trim_matches('"').to_string();
             if let Ok(mut state) = self.state.lock() {
                state.recent_logs.push(format!("{:?}: {}", *event.metadata().level(), clean_msg));
                if state.recent_logs.len() > 100 {
                    state.recent_logs.remove(0);
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Shared Application State for TUI
    let app_state = Arc::new(Mutex::new(AppState::new()));

    // Initialize Logging (File + TUI)
    let file_appender = tracing_appender::rolling::daily("logs", "bot.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false);
    
    // Filter noise: Only show info+ for our crates, warn+ for others
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,engine=info,strategy=info,executor=info"));

    let tui_layer = TuiLogLayer { state: app_state.clone() };

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(tui_layer)
        .init();

    info!("Starting Solana MEV Bot Engine (Phase 9: TUI Integration)...");

    // 1. Load Secure Configuration
    let config = match BotConfig::new() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("Failed to load configuration: {}. Ensure .env is present.", e);
            return Ok(());
        }
    };

    let runtime = Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .thread_name("mev-worker")
        .build()?;

    // Spawn TUI in a separate thread (Main thread blocks on this later?)
    // Actually, TUI needs to be on the main thread for terminal handling usually, 
    // or we spawn the runtime in background. Let's spawn runtime in background.
    
    // We clone state for the async tasks
    let state_for_tasks = app_state.clone();
    
    runtime.spawn(async move {
        // 2. Initialize Black Box Recorder
        let recorder: Arc<AsyncCsvWriter> = match AsyncCsvWriter::new(&config.data_output_dir).await {
            Ok(r) => Arc::new(r),
            Err(e) => {
                error!("Failed to initialize data recorder: {}", e);
                return;
            }
        };

        // 3. Initialize Communication Pipeline (MPSC)
        let (pool_tx, mut pool_rx) = mpsc::channel::<PoolUpdate>(5000 * 3); 

        // 4. Initialize Executor (Jito Client)
        let keypair = if config.private_key == "[YOUR_PRIVATE_KEY_HERE]" || config.private_key.is_empty() {
             let kp = Keypair::new();
             warn!("Using EPHEMERAL keypair. whitelist will fail on restart!");
             Arc::new(kp)
        } else {
             let decoded_result: Result<Vec<u8>, _> = bs58::decode(&config.private_key).into_vec();
             match decoded_result {
                 Ok(key_bytes) => extract_keypair_from_bytes(&key_bytes).unwrap_or_else(|_| {
                     error!("Invalid private key in .env. Generating ephemeral.");
                     Arc::new(Keypair::new())
                 }),
                 Err(_) => {
                     error!("Failed to base58 decode private key. Generating ephemeral.");
                     Arc::new(Keypair::new())
                 }
             }
        };
        info!("Searcher Identity: {}", keypair.pubkey());

        let jito_client = match JitoClient::new(&config.block_engine_url, keypair.clone()).await {
            Ok(client) => Some(Arc::new(client)),
            Err(e) => {
                if config.dry_run {
                    warn!("Failed to initialize Jito Client: {}. Running in DATA COLLECTION ONLY mode.", e);
                    None
                } else {
                    error!("Failed to initialize Jito Client: {}. Ensure your network allows gRPC.", e);
                    return;
                }
            }
        };

        // 5. Initialize Simulator
        let simulator = {
            let rpc = Arc::new(solana_client::rpc_client::RpcClient::new(config.rpc_url.clone()));
            Arc::new(simulation::Simulator::new(rpc))
        };

        // 6. Initialize Strategy Engine
        let model_path = std::env::var("MODEL_PATH").ok();
        let strategy_engine = Arc::new(StrategyEngine::new(
            jito_client.clone(), 
            Some(simulator),
            model_path.as_deref()
        ));

        // 6. Start Market Listeners
        let (cb_tx, cb_rx) = crossbeam::channel::unbounded::<PoolUpdate>();

        for addr_str in config.pools.split(',') {
            let addr: &str = addr_str.trim();
            if addr.is_empty() { continue; }
            if let Ok(pool_pubkey) = addr.parse::<Pubkey>() {
                info!("Launching listener for pool: {}", pool_pubkey);
                start_listener(config.rpc_url.clone(), pool_pubkey, cb_tx.clone());
                
                // Track pool count for TUI
                if let Ok(mut s) = state_for_tasks.lock() {
                    s.pool_count += 1;
                }
            } else {
                error!("Invalid pool address in config: {}", addr);
            }
        }

        // 7. Data Bridge & Recorder
        let pool_tx_bridge = pool_tx.clone();
        let recorder_handle = recorder.clone();
        let cb_rx_bridge = cb_rx.clone();
        
        tokio::spawn(async move {
            info!("Data Bridge & Recorder active.");
            while let Ok(update) = cb_rx_bridge.recv() {
                let r = recorder_handle.clone();
                let u: PoolUpdate = update.clone();
                tokio::spawn(async move { r.record(u).await });

                if let Err(e) = pool_tx_bridge.send(update).await {
                    error!("Channel bridge failed: {}", e);
                    break;
                }
            }
        });

        // 7.5 Simulation Mode
        if config.simulation {
            let pool_tx_sim = pool_tx.clone();
            let recorder_sim = recorder.clone();
            tokio::spawn(async move {
                info!("SIMULATION MODE ACTIVE: Generating mock market events...");
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
                let pool_addr = "58oQChGsNrtmhaJSRph38tB3BwpL66F42FMa86Fv3Gry".parse::<Pubkey>().unwrap();
                let mut reserve_a = 1000_000_000u128;
                let mut reserve_b = 150_000_000_000u128;
                
                loop {
                    interval.tick().await;
                    let is_buy = rand::random::<bool>();
                    let trade_size = (rand::random::<u64>() % 50_000_000) as u128;

                    if is_buy {
                        reserve_a += trade_size;
                        let k = 1000_000_000 * 150_000_000_000u128;
                        reserve_b = k / reserve_a;
                    } else {
                        reserve_a -= trade_size;
                        let k = 1000_000_000 * 150_000_000_000u128;
                        reserve_b = k / reserve_a;
                    }

                    let mock_update = PoolUpdate {
                        pool_address: pool_addr,
                        program_id: mev_core::constants::RAYDIUM_V4_PROGRAM,
                        mint_a: "So11111111111111111111111111111111111111112".parse().unwrap(),
                        mint_b: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse().unwrap(),
                        reserve_a,
                        reserve_b,
                        fee_bps: 30,
                        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                    };

                    recorder_sim.record(mock_update.clone()).await;
                    if let Err(e) = pool_tx_sim.send(mock_update).await {
                        error!("Simulation send failed: {}", e);
                        break;
                    }
                }
            });
        }

        // 8. Strategy & Execution Loop
        let strategy_handle = strategy_engine.clone();
        let is_dry_run = config.dry_run;
        let pnl_metric = strategy_engine.total_simulated_pnl.clone();
        let state_metric = state_for_tasks.clone();
        let arb_recorder = recorder.clone();

        tokio::spawn(async move {
            info!("AI Strategy Engine loop active (DRY_RUN={}).", is_dry_run);
            while let Some(update) = pool_rx.recv().await {
                match strategy_handle.process_event(update).await {
                    Ok(Some(opp)) => {
                        // Record arbitrage opportunity for AI training
                        arb_recorder.record_arbitrage(opp.clone()).await;
                        
                        // Opportunity executed/simulated! Update TUI.
                         if let Ok(mut s) = state_metric.lock() {
                            s.recent_opportunities.push(opp);
                            if s.recent_opportunities.len() > 50 {
                                s.recent_opportunities.remove(0);
                            }
                        }
                    },
                    Ok(None) => {}, // No opportunity or execution disabled/failed
                    Err(e) => error!("Strategy processing error: {}", e),
                }
                
                // Update TUI PnL
                let pnl = pnl_metric.load(std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut s) = state_metric.lock() {
                    s.total_simulated_pnl = pnl;
                }
            }
        });

        info!("System Hot. Recording to {} | DRY_RUN={}", config.data_output_dir, is_dry_run);
    });

    // Run TUI on Main Thread
    let mut tui_app = TuiApp::new(app_state);
    tui_app.run()?;

    Ok(())
}

fn extract_keypair_from_bytes(bytes: &[u8]) -> Result<Arc<Keypair>, Box<dyn std::error::Error>> {
    let kp = Keypair::from_bytes(bytes)?;
    Ok(Arc::new(kp))
}

