use crate::models::{Alert, AlertConfig, Transaction};
use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use teloxide::prelude::*;
use uuid::Uuid;
use bigdecimal::ToPrimitive;


pub async fn check_and_send_alerts(
    tx: &Transaction,
    config: &AlertConfig,
    alert_sender: &tokio::sync::broadcast::Sender<Alert>,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let mut alerts = Vec::new();
    
    if let Some(amount) = tx.amount.to_f64() {
        if amount > config.large_tx_threshold {
            let alert = Alert {
                id: Uuid::new_v4(),
                alert_type: "large_transaction".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "Large {} transaction: ${:.2} from {}",
                    tx.tx_type, amount, &tx.from_address[..10]
                ),
                tx_hash: Some(tx.tx_hash.clone()),
                metadata: Some(serde_json::json!({
                    "amount": amount,
                    "type": tx.tx_type,
                })),
                acknowledged: false,
                timestamp: chrono::Utc::now(),
            };
            alerts.push(alert);
        }
    }
    
    if config.failed_tx_enabled && tx.status == "failed" {
        let alert = Alert {
            id: Uuid::new_v4(),
            alert_type: "failed_transaction".to_string(),
            severity: "error".to_string(),
            message: format!(
                "Transaction failed: {} - {}",
                tx.tx_type, &tx.tx_hash[..10]
            ),
            tx_hash: Some(tx.tx_hash.clone()),
            metadata: None,
            acknowledged: false,
            timestamp: chrono::Utc::now(),
        };
        alerts.push(alert);
    }
    
    for alert in alerts {
        if config.email_enabled {
            if let Err(e) = send_email_alert(&alert, config).await {
                tracing::error!("Failed to send email: {}", e);
            }
        }
        
        if config.telegram_enabled {
            if let Err(e) = send_telegram_alert(&alert, config).await {
                tracing::error!("Failed to send telegram: {}", e);
            }
        }
        
        crate::database::insert_alert(pool, &alert).await?;
        let _ = alert_sender.send(alert);
    }
    
    Ok(())
}

async fn send_email_alert(alert: &Alert, config: &AlertConfig) -> Result<()> {
    let smtp_host = std::env::var("SMTP_HOST")?;
    let smtp_port = std::env::var("SMTP_PORT")?;
    let smtp_user = std::env::var("SMTP_USERNAME")?;
    let smtp_pass = std::env::var("SMTP_PASSWORD")?;
    
    for recipient in &config.email_recipients {
        let email = Message::builder()
            .from(format!("Mantle Monitor <{}>", smtp_user).parse()?)
            .to(recipient.parse()?)
            .subject(format!("Mantle Alert: {}", alert.alert_type))
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Alert: {}\nSeverity: {}\nMessage: {}\nTx: {}\nTime: {}",
                alert.alert_type,
                alert.severity,
                alert.message,
                alert.tx_hash.as_deref().unwrap_or("N/A"),
                alert.timestamp
            ))?;
        
        let creds = Credentials::new(smtp_user.clone(), smtp_pass.clone());
        let mailer = SmtpTransport::relay(&smtp_host)?
            .port(smtp_port.parse()?)
            .credentials(creds)
            .build();
        
        mailer.send(&email)?;
    }
    
    Ok(())
}

async fn send_telegram_alert(alert: &Alert, config: &AlertConfig) -> Result<()> {
    if let Some(chat_id) = config.telegram_chat_id {
        let bot = Bot::from_env();
        
        let message = format!(
            " *Mantle Alert*\n\n\
            *Type:* {}\n\
            *Severity:* {}\n\
            *Message:* {}\n\
            *Time:* {}\n\
            *TX:* `{}`",
            alert.alert_type,
            alert.severity,
            alert.message,
            alert.timestamp.format("%Y-%m-%d %H:%M:%S"),
            alert.tx_hash.as_deref().unwrap_or("N/A")
        );
        
        bot.send_message(ChatId(chat_id), message)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
    }
    
    Ok(())
}