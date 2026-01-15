use ethers::prelude::*;
use std::sync::Arc;
use anyhow::Result;
use tokio::sync::broadcast;
use crate::models::Transaction as MonitoredTx;
use bigdecimal::BigDecimal;
use std::str::FromStr;

pub async fn connect_provider(wss_url: &str) -> Result<Provider<Ws>> {
    let provider = Provider::<Ws>::connect(wss_url).await?;
    tracing::info!("Connected to Mantle blockchain");
    Ok(provider)
}

pub async fn monitor_blocks(
    provider: Arc<Provider<Ws>>,
    tx_sender: broadcast::Sender<MonitoredTx>,
    pool: sqlx::PgPool,
) -> Result<()> {
    let mut stream = provider.subscribe_blocks().await?;
    
    tracing::info!("Monitoring Mantle blockchain...");
    
    while let Some(block) = stream.next().await {
        if let Some(block_number) = block.number {
            if let Err(e) = process_block(&provider, block_number, &tx_sender, &pool).await {
                tracing::error!("Error processing block {}: {}", block_number, e);
            }
        }
    }
    
    Ok(())
}

async fn process_block(
    provider: &Provider<Ws>,
    block_number: U64,
    tx_sender: &broadcast::Sender<MonitoredTx>,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let block = provider
        .get_block_with_txs(block_number)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
    
    tracing::debug!("📦 Processing block {}", block_number);
    
    for tx in &block.transactions {
        if let Ok(Some(receipt)) = provider.get_transaction_receipt(tx.hash).await {
            let monitored_tx = create_monitored_tx(tx, &receipt, &block);
            
            if let Err(e) = crate::database::insert_transaction(pool, &monitored_tx).await {
                tracing::error!("Failed to save tx: {}", e);
                continue;
            }
            
            let _ = tx_sender.send(monitored_tx);
        }
    }
    
    Ok(())
}

fn create_monitored_tx(
    tx: &Transaction,
    receipt: &TransactionReceipt,
    block: &Block<Transaction>,
) -> MonitoredTx {
    let tx_type = classify_transaction(tx);
    let status = if receipt.status == Some(U64::from(1)) {
        "success"
    } else {
        "failed"
    };
    
    MonitoredTx {
        id: uuid::Uuid::new_v4(),
        tx_hash: format!("{:?}", tx.hash),
        tx_type,
        status: status.to_string(),
        from_address: format!("{:?}", tx.from),
        to_address: tx.to.map(|a| format!("{:?}", a)).unwrap_or_default(),
        amount: BigDecimal::from_str(&tx.value.to_string()).unwrap_or_default(),
        gas_used: receipt.gas_used.unwrap_or_default().as_u64() as i64,
        gas_price: tx.gas_price.unwrap_or_default().as_u64() as i64,
        block_number: block.number.unwrap_or_default().as_u64() as i64,
        block_timestamp: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn classify_transaction(tx: &Transaction) -> String {
    if tx.input.len() >= 4 {
        let selector = format!(
            "{:02x}{:02x}{:02x}{:02x}", 
            tx.input[0], tx.input[1], tx.input[2], tx.input[3]
        );
        
        match selector.as_str() {
            "38ed1739" => "swap".to_string(),
            "a694fc3a" => "stake".to_string(),
            "c5eabedf" => "lend".to_string(),
            "e8eda9df" => "borrow".to_string(),
            "e2bbb158" => "deposit".to_string(),
            _ => "transfer".to_string(),
        }
    } else {
        "transfer".to_string()
    }
}