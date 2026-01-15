-- Monitored Transactions Table
CREATE TABLE monitored_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tx_hash VARCHAR(66) UNIQUE NOT NULL,
    tx_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    amount DECIMAL(78, 0) NOT NULL,
    gas_used BIGINT NOT NULL,
    gas_price BIGINT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_tx_hash ON monitored_transactions(tx_hash);
CREATE INDEX idx_status ON monitored_transactions(status);
CREATE INDEX idx_tx_type ON monitored_transactions(tx_type);
CREATE INDEX idx_block_timestamp ON monitored_transactions(block_timestamp DESC);
CREATE INDEX idx_block_number ON monitored_transactions(block_number DESC);
CREATE INDEX idx_from_address ON monitored_transactions(from_address);
CREATE INDEX idx_to_address ON monitored_transactions(to_address);

-- Alerts Table
CREATE TABLE alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    message TEXT NOT NULL,
    tx_hash VARCHAR(66),
    metadata JSONB,
    acknowledged BOOLEAN DEFAULT FALSE,
    timestamp TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_alert_timestamp ON alerts(timestamp DESC);
CREATE INDEX idx_alert_type ON alerts(alert_type);
CREATE INDEX idx_alert_acknowledged ON alerts(acknowledged);

-- Analytics Views
CREATE VIEW hourly_stats AS
SELECT 
    date_trunc('hour', block_timestamp) as hour,
    COUNT(*) as tx_count,
    COUNT(*) FILTER (WHERE status = 'success') as success_count,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
    AVG(gas_used) as avg_gas,
    SUM(amount) as total_volume,
    COUNT(DISTINCT from_address) as unique_senders
FROM monitored_transactions
GROUP BY date_trunc('hour', block_timestamp)
ORDER BY hour DESC;

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger for auto-updating timestamps
CREATE TRIGGER update_monitored_transactions_updated_at 
    BEFORE UPDATE ON monitored_transactions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();