mod api;
mod alerts;
mod blockchain;
mod database;
mod models;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mantle_monitor=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    tracing::info!(" Starting Mantle Transaction Monitor");

    // Configuration
    let database_url = std::env::var("DATABASE_URL")?;
    let wss_rpc = std::env::var("MANTLE_WSS_RPC")?;
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // Alert configuration
    let alert_config = models::AlertConfig {
        large_tx_threshold: std::env::var("ALERT_LARGE_TX_THRESHOLD")
            .unwrap_or_else(|_| "5000.0".to_string())
            .parse()?,
        failed_tx_enabled: std::env::var("ALERT_FAILED_TX_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()?,
        anomaly_detection: std::env::var("ALERT_ANOMALY_DETECTION")
            .unwrap_or_else(|_| "true".to_string())
            .parse()?,
        email_enabled: std::env::var("ALERT_EMAIL_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()?,
        telegram_enabled: std::env::var("ALERT_TELEGRAM_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()?,
        email_recipients: std::env::var("ALERT_EMAIL_RECIPIENTS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.to_string())
            .collect(),
        telegram_chat_id: std::env::var("TELEGRAM_CHAT_ID")
            .ok()
            .and_then(|s| s.parse().ok()),
    };
    let alert_config = Arc::new(tokio::sync::RwLock::new(alert_config));

    // Setup database
    let db = database::create_pool(&database_url).await?;
    tracing::info!(" Database connected");

    // Create broadcast channels
    let (tx_sender, _) = broadcast::channel(1000);
    let (alert_sender, _) = broadcast::channel(1000);

    // Create application state
    let state = api::AppState {
        db: db.clone(),
        tx_sender: tx_sender.clone(),
        alert_sender: alert_sender.clone(),
    };

    // Connect to Mantle blockchain
    let provider = blockchain::connect_provider(&wss_rpc).await?;
    let provider = Arc::new(provider);

    // Start blockchain monitor in background
    let monitor_provider = provider.clone();
    let monitor_tx_sender = tx_sender.clone();
    let monitor_alert_sender = alert_sender.clone();
    let monitor_db = db.clone();
    let monitor_config = alert_config.clone();
    
    tokio::spawn(async move {
        loop {
            match blockchain::monitor_blocks(
                monitor_provider.clone(),
                monitor_tx_sender.clone(),
                monitor_db.clone(),
            ).await {
                Ok(_) => {},
                Err(e) => {
                    tracing::error!("Blockchain monitor error: {}", e);
                    tracing::info!("Reconnecting in 5 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

    // Build and start API server
    let app = api::create_router(state);
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("API Documentation: http://{}/api/health", addr);
    tracing::info!("WebSocket: ws://{}/ws", addr);

    axum::serve(listener, app).await?;

    Ok(())
}