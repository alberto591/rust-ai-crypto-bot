pub use mev_core::telemetry::*;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use prometheus::{TextEncoder, Encoder};

/// Start metrics HTTP server
pub async fn serve_metrics() {
    let port = std::env::var("METRICS_PORT")
        .unwrap_or_else(|_| "8082".to_string())
        .parse::<u16>()
        .unwrap_or(8082);

    tracing::info!("📊 Prometheus metrics server starting on 0.0.0.0:{}", port);

    let app = Router::new().route("/metrics", get(move || async {
        let encoder = TextEncoder::new();
        let metric_families = REGISTRY.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }));

    tokio::spawn(async move {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                if let Err(e) = axum::serve(listener, app).await {
                    tracing::error!("❌ Metrics server error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to start metrics server on {}: {}", addr, e);
            }
        }
    });
}
