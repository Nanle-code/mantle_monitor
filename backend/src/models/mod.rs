use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub tx_hash: String,
    pub tx_type: String,
    pub status: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: BigDecimal,
    pub gas_used: i64,
    pub gas_price: i64,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alert {
    pub id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub tx_hash: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub acknowledged: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionStats {
    pub total_txs: i64,
    pub success_rate: f64,
    pub failed_rate: f64,
    pub avg_gas_used: f64,
    pub total_volume: String,
    pub active_users: i64,
    pub tps: f64,
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub status: Option<String>,
    pub tx_type: Option<String>,
    pub from_block: Option<i64>,
    pub to_block: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub large_tx_threshold: f64,
    pub failed_tx_enabled: bool,
    pub anomaly_detection: bool,
    pub email_enabled: bool,
    pub telegram_enabled: bool,
    pub email_recipients: Vec<String>,
    pub telegram_chat_id: Option<i64>,
}