use axum::{
    extract::{State, Query, Path, ws::{WebSocket, WebSocketUpgrade}},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use crate::models::{Transaction, TransactionStats, Alert, QueryParams};
use tower_http::cors::CorsLayer;
use sqlx::Row;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub tx_sender: tokio::sync::broadcast::Sender<Transaction>,
    pub alert_sender: tokio::sync::broadcast::Sender<Alert>,
}

// pub fn create_router(state: AppState) -> Router {
//     Router::new()
//         .route("/api/health", get(health_check))
//         .route("/api/transactions", get(get_transactions))
//         .route("/api/transactions/:hash", get(get_transaction_by_hash))
//         .route("/api/stats", get(get_stats))
//         .route("/api/alerts", get(get_alerts))
//         .route("/api/export/csv", get(export_csv))
//         .route("/ws", get(websocket_handler))
//         .layer(CorsLayer::permissive())
//         .with_state(state)
// }

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/transactions", get(get_transactions))
        .route("/api/transactions/{hash}", get(get_transaction_by_hash))
        .route("/api/stats", get(get_stats))
        .route("/api/alerts", get(get_alerts))
        .route("/api/export/csv", get(export_csv))
        .route("/ws", get(websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}


async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    }))
}

async fn get_transactions(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<Transaction>>, StatusCode> {
    let limit = params.limit.unwrap_or(50).min(500);
    let offset = params.offset.unwrap_or(0);
    
    tracing::info!("Fetching transactions with status: {:?}, type: {:?}", params.status, params.tx_type);
    
    let txs = crate::database::get_transactions(
        &state.db,
        params.status,
        params.tx_type,
        limit,
        offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to get transactions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    Ok(Json(txs))
}

async fn get_transaction_by_hash(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Json<Transaction>, StatusCode> {
    let row = sqlx::query(
        "SELECT * FROM monitored_transactions WHERE tx_hash = $1"
    )
    .bind(&hash)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let tx = Transaction {
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
    };
    
    Ok(Json(tx))
}

async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<TransactionStats>, StatusCode> {
    let stats = crate::database::get_stats(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(stats))
}

async fn get_alerts(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<Alert>>, StatusCode> {
    let limit = params.limit.unwrap_or(50).min(500);
    
    let alerts = crate::database::get_alerts(&state.db, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(alerts))
}

async fn export_csv(
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    let txs = crate::database::get_transactions(
        &state.db, None, None, 10000, 0
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut wtr = csv::Writer::from_writer(vec![]);
    
    for tx in txs {
        wtr.serialize(tx)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    
    let data = wtr.into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "text/csv"),
            ("Content-Disposition", "attachment; filename=pharos_transactions.csv")
        ],
        data,
    ).into_response())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    let mut tx_rx = state.tx_sender.subscribe();
    let mut alert_rx = state.alert_sender.subscribe();
    
    loop {
        tokio::select! {
            Ok(tx) = tx_rx.recv() => {
                let msg = serde_json::json!({
                    "type": "transaction",
                    "data": tx
                });
                if socket.send(axum::extract::ws::Message::Text(
                    msg.to_string().into()
                )).await.is_err() {
                    break;
                }
            }
            Ok(alert) = alert_rx.recv() => {
                let msg = serde_json::json!({
                    "type": "alert",
                    "data": alert
                });
                if socket.send(axum::extract::ws::Message::Text(
                    msg.to_string().into()
                )).await.is_err() {
                    break;
                }
            }
        }
    }
}