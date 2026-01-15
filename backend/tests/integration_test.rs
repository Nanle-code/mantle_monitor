#[cfg(test)]
mod tests {
    use pharos_monitor::*;
    
    #[tokio::test]
    async fn test_database_connection() {
        let pool = database::create_pool("postgres://localhost/test").await;
        assert!(pool.is_ok());
    }
    
    #[tokio::test]
    async fn test_transaction_classification() {
        // Test transaction type detection
    }
    
    #[tokio::test]
    async fn test_alert_thresholds() {
        // Test alert triggers
    }
}