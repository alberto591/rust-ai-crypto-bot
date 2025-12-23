use tokio::sync::mpsc;
use tokio::io::AsyncWriteExt;
use tokio::fs::OpenOptions;
use chrono::Utc;

pub struct PerformanceTracker {
    sender: mpsc::Sender<String>,
}

impl PerformanceTracker {
    /// Spawns a background task to handle file I/O safely
    pub async fn new(file_path: &str) -> Self {
        let (tx, mut rx) = mpsc::channel::<String>(100); // Buffer 100 logs
        let path = file_path.to_string();

        tokio::spawn(async move {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .expect("Failed to open log file");

            while let Some(log_entry) = rx.recv().await {
                if let Err(e) = file.write_all(log_entry.as_bytes()).await {
                    eprintln!("❌ Logger Error: {}", e);
                }
            }
        });

        Self { sender: tx }
    }

    pub async fn log_trade(&self, token: &str, profit: i64, mode: &str) {
        let timestamp = Utc::now().to_rfc3339();
        let log_entry = format!("{},{},{},{}\n", timestamp, token, profit, mode);
        
        // Non-blocking send. If buffer full, we drop log rather than crash app (HFT preference)
        let _ = self.sender.try_send(log_entry);
    }
}
