use mev_core::{PoolUpdate, ArbitrageOpportunity};
use tokio::fs::{OpenOptions, create_dir_all};
use tokio::io::AsyncWriteExt;
use std::path::Path;
use tracing::{info, error};

pub struct AsyncCsvWriter {
    pool_data_path: String,
    arbitrage_data_path: String,
}

impl AsyncCsvWriter {
    pub async fn new(output_dir: &str) -> Result<Self, std::io::Error> {
        let path = Path::new(output_dir);
        if !path.exists() {
            create_dir_all(path).await?;
        }
        
        let pool_data_path = format!("{}/market_data.csv", output_dir);
        let arbitrage_data_path = format!("{}/arbitrage_data.csv", output_dir);
        
        // Write CSV headers if files don't exist
        if !Path::new(&arbitrage_data_path).exists() {
            let header = "timestamp,num_hops,profit_lamports,input_amount,total_fees_bps,max_price_impact_bps,min_liquidity,route\n";
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&arbitrage_data_path)
                .await?;
            file.write_all(header.as_bytes()).await?;
        }
        
        Ok(Self { pool_data_path, arbitrage_data_path })
    }

    pub async fn record(&self, update: PoolUpdate) {
        let line = format!(
            "{},{},{},{},{},{}\\n",
            update.timestamp,
            update.pool_address,
            update.program_id,
            update.reserve_a,
            update.reserve_b,
            if update.reserve_a > 0 { (update.reserve_b as f64 / update.reserve_a as f64).to_string() } else { "0".to_string() }
        );

        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.pool_data_path)
            .await 
        {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to open pool data CSV for recording: {}", e);
                return;
            }
        };

        if let Err(e) = file.write_all(line.as_bytes()).await {
            error!("Failed to write to pool data CSV: {}", e);
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
            "{},{},{},{},{},{},{},\"{}\"\\n",
            opp.timestamp,
            opp.steps.len(),
            opp.expected_profit_lamports,
            opp.input_amount,
            opp.total_fees_bps,
            opp.max_price_impact_bps,
            opp.min_liquidity,
            route
        );

        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.arbitrage_data_path)
            .await 
        {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to open arbitrage data CSV for recording: {}", e);
                return;
            }
        };

        if let Err(e) = file.write_all(line.as_bytes()).await {
            error!("Failed to write to arbitrage data CSV: {}", e);
        }
    }
}
