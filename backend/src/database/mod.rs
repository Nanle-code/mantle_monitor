use crate::models::{Alert, Transaction, TransactionStats};
use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions, Row};

pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;
    
    tracing::info!("Database connection pool created");
    Ok(pool)
}

pub async fn insert_transaction(
    pool: &PgPool,
    tx: &Transaction,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO monitored_transactions 
        (id, tx_hash, tx_type, status, from_address, to_address, 
         amount, gas_used, gas_price, block_number, block_timestamp)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (tx_hash) DO NOTHING
        "#
    )
    .bind(&tx.id)
    .bind(&tx.tx_hash)
    .bind(&tx.tx_type)
    .bind(&tx.status)
    .bind(&tx.from_address)
    .bind(&tx.to_address)
    .bind(&tx.amount)
    .bind(tx.gas_used)
    .bind(tx.gas_price)
    .bind(tx.block_number)
    .bind(tx.block_timestamp)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn get_transactions(
    pool: &PgPool,
    status: Option<String>,
    tx_type: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Transaction>> {
    let mut query = String::from(
        "SELECT * FROM monitored_transactions WHERE 1=1"
    );
    
    if let Some(s) = status {
        query.push_str(&format!(" AND status = '{}'", s));
    }
    
    if let Some(t) = tx_type {
        query.push_str(&format!(" AND tx_type = '{}'", t));
    }
    
    query.push_str(&format!(
        " ORDER BY block_timestamp DESC LIMIT {} OFFSET {}",
        limit, offset
    ));
    
    let rows = sqlx::query(&query)
        .fetch_all(pool)
        .await?;
    
    let txs = rows.into_iter().map(|row| Transaction {
        id: row.get("id"),
        tx_hash: row.get("tx_hash"),
        tx_type: row.get("tx_type"),
        status: row.get("status"),
        from_address: row.get("from_address"),
        to_address: row.get("to_address"),
        amount: row.get("amount"),
        gas_used: row.get("gas_used"),
        gas_price: row.get("gas_price"),
        block_number: row.get("block_number"),
        block_timestamp: row.get("block_timestamp"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }).collect();
    
    Ok(txs)
}

pub async fn get_stats(pool: &PgPool) -> Result<TransactionStats> {
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_txs,
            (COUNT(*) FILTER (WHERE status = 'success')::float / 
             NULLIF(COUNT(*), 0) * 100) as success_rate,
            (COUNT(*) FILTER (WHERE status = 'failed')::float / 
             NULLIF(COUNT(*), 0) * 100) as failed_rate,
            AVG(gas_used) as avg_gas,
            SUM(CAST(amount AS DECIMAL)) as total_vol,
            COUNT(DISTINCT from_address) as active_users
        FROM monitored_transactions
        WHERE block_timestamp > NOW() - INTERVAL '1 hour'
        "#
    )
    .fetch_one(pool)
    .await?;
    
    let total_txs: i64 = row.get("total_txs");
    let success_rate: Option<f64> = row.try_get("success_rate").ok();
    let failed_rate: Option<f64> = row.try_get("failed_rate").ok();
    let avg_gas: Option<f64> = row.try_get("avg_gas").ok();
    let total_vol: Option<String> = row.try_get::<Option<f64>, _>("total_vol")
        .ok()
        .flatten()
        .map(|v| v.to_string());
    let active_users: i64 = row.get("active_users");
    
    let tps = total_txs as f64 / 3600.0;
    
    Ok(TransactionStats {
        total_txs,
        success_rate: success_rate.unwrap_or(0.0),
        failed_rate: failed_rate.unwrap_or(0.0),
        avg_gas_used: avg_gas.unwrap_or(0.0),
        total_volume: total_vol.unwrap_or_else(|| "0".to_string()),
        active_users,
        tps,
    })
}

pub async fn insert_alert(pool: &PgPool, alert: &Alert) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO alerts 
        (id, alert_type, severity, message, tx_hash, metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(&alert.id)
    .bind(&alert.alert_type)
    .bind(&alert.severity)
    .bind(&alert.message)
    .bind(&alert.tx_hash)
    .bind(&alert.metadata)
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn get_alerts(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<Alert>> {
    let rows = sqlx::query(
        "SELECT * FROM alerts ORDER BY timestamp DESC LIMIT $1"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    
    let alerts = rows.into_iter().map(|row| Alert {
        id: row.get("id"),
        alert_type: row.get("alert_type"),
        severity: row.get("severity"),
        message: row.get("message"),
        tx_hash: row.try_get("tx_hash").ok(),
        metadata: row.try_get("metadata").ok(),
        acknowledged: row.get("acknowledged"),
        timestamp: row.get("timestamp"),
    }).collect();
    
    Ok(alerts)
}