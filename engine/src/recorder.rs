use mev_core::{PoolUpdate, ArbitrageOpportunity};
use tokio::fs::{OpenOptions, create_dir_all, File};
use tokio::io::{AsyncWriteExt, BufWriter};
use std::path::Path;
use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AsyncCsvWriter {
    pool_writer: Arc<Mutex<BufWriter<File>>>,
    arbitrage_writer: Arc<Mutex<BufWriter<File>>>,
}

impl AsyncCsvWriter {
    pub async fn new(output_dir: &str) -> Result<Self, std::io::Error> {
        let path = Path::new(output_dir);
        if !path.exists() {
            create_dir_all(path).await?;
        }
        
        let pool_data_path = format!("{}/market_data.csv", output_dir);
        let arbitrage_data_path = format!("{}/arbitrage_data.csv", output_dir);
        
        // 1. Prepare Pool Data Writer
        let pool_exists = Path::new(&pool_data_path).exists();
        let pool_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&pool_data_path)
            .await?;
        let mut pool_writer = BufWriter::new(pool_file);
        
        if !pool_exists {
            let header = "timestamp,pool_address,program_id,reserve_a,reserve_b,price_ratio\n";
            pool_writer.write_all(header.as_bytes()).await?;
            pool_writer.flush().await?;
        }

        // 2. Prepare Arbitrage Data Writer
        let arb_exists = Path::new(&arbitrage_data_path).exists();
        let arb_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&arbitrage_data_path)
            .await?;
        let mut arb_writer = BufWriter::new(arb_file);

        if !arb_exists {
            let header = "timestamp,num_hops,profit_lamports,input_amount,total_fees_bps,max_price_impact_bps,min_liquidity,route\n";
            arb_writer.write_all(header.as_bytes()).await?;
            arb_writer.flush().await?;
        }

        info!("✅ Data Recorder initialized at {}", output_dir);

        Ok(Self { 
            pool_writer: Arc::new(Mutex::new(pool_writer)),
            arbitrage_writer: Arc::new(Mutex::new(arb_writer)),
        })
    }

    pub async fn record(&self, update: PoolUpdate) {
        let line = format!(
            "{},{},{},{},{},{}\n",
            update.timestamp,
            update.pool_address,
            update.program_id,
            update.reserve_a,
            update.reserve_b,
            if update.reserve_a > 0 { (update.reserve_b as f64 / update.reserve_a as f64).to_string() } else { "0".to_string() }
        );

        let mut writer = self.pool_writer.lock().await;
        if let Err(e) = writer.write_all(line.as_bytes()).await {
            error!("Failed to write to pool data CSV: {}", e);
        }
        // Periodic flush could be added here or relied on buffer capacity
        if let Err(e) = writer.flush().await {
             error!("Failed to flush pool data CSV: {}", e);
        }
    }
    
    pub async fn record_arbitrage(&self, opp: ArbitrageOpportunity) {
        // Build route string (mint addresses abbreviated)
        let route: String = opp.steps.iter()
            .map(|s| {
                let m = s.input_mint.to_string();
                format!("{}..", &m[0..4.min(m.len())])
            })
            .collect::<Vec<_>>()
            .join("->");
        
        let line = format!(
            "{},{},{},{},{},{},{},\"{}\"\n",
            opp.timestamp,
            opp.steps.len(),
            opp.expected_profit_lamports,
            opp.input_amount,
            opp.total_fees_bps,
            opp.max_price_impact_bps,
            opp.min_liquidity,
            route
        );

        let mut writer = self.arbitrage_writer.lock().await;
        if let Err(e) = writer.write_all(line.as_bytes()).await {
            error!("Failed to write to arbitrage data CSV: {}", e);
        }
        if let Err(e) = writer.flush().await {
            error!("Failed to flush arbitrage data CSV: {}", e);
        }
    }
}
