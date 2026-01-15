-- Create the Mantle indexer database and user
CREATE DATABASE mantle_indexer;
CREATE USER mantle_user WITH PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE mantle_indexer TO mantle_user;

-- Connect to the new database
\c mantle_indexer

-- Grant schema privileges
GRANT ALL ON SCHEMA public TO mantle_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO mantle_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO mantle_user;

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- =============================================================================
-- CORE TABLES
-- =============================================================================

-- Blocks Table (for tracking indexed blocks)
CREATE TABLE blocks (
    block_number BIGINT PRIMARY KEY,
    block_hash VARCHAR(66) UNIQUE NOT NULL,
    parent_hash VARCHAR(66) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    transaction_count INT NOT NULL,
    gas_used BIGINT NOT NULL,
    gas_limit BIGINT NOT NULL,
    base_fee_per_gas BIGINT,
    indexed_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_blocks_timestamp ON blocks(timestamp DESC);
CREATE INDEX idx_blocks_indexed_at ON blocks(indexed_at DESC);

-- Monitored Transactions Table (Enhanced)
CREATE TABLE monitored_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tx_hash VARCHAR(66) UNIQUE NOT NULL,
    tx_index INT NOT NULL,
    block_number BIGINT NOT NULL REFERENCES blocks(block_number),
    block_timestamp TIMESTAMPTZ NOT NULL,
    
    -- Transaction Details
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42),
    value NUMERIC(78, 0) NOT NULL DEFAULT 0,
    input_data TEXT,
    nonce BIGINT NOT NULL,
    
    -- Gas Details
    gas_limit BIGINT NOT NULL,
    gas_used BIGINT,
    gas_price BIGINT,
    max_fee_per_gas BIGINT,
    max_priority_fee_per_gas BIGINT,
    effective_gas_price BIGINT,
    
    -- Status and Type
    status VARCHAR(20) NOT NULL,
    tx_type VARCHAR(50) NOT NULL,
    
    -- Contract Interaction
    contract_address VARCHAR(42),
    method_signature VARCHAR(10),
    decoded_method VARCHAR(100),
    
    -- Additional Metadata
    error_message TEXT,
    metadata JSONB DEFAULT '{}',
    
    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_tx_hash ON monitored_transactions(tx_hash);
CREATE INDEX idx_tx_status ON monitored_transactions(status);
CREATE INDEX idx_tx_type ON monitored_transactions(tx_type);
CREATE INDEX idx_tx_block_timestamp ON monitored_transactions(block_timestamp DESC);
CREATE INDEX idx_tx_block_number ON monitored_transactions(block_number DESC);
CREATE INDEX idx_tx_from_address ON monitored_transactions(from_address);
CREATE INDEX idx_tx_to_address ON monitored_transactions(to_address);
CREATE INDEX idx_tx_contract_address ON monitored_transactions(contract_address);
CREATE INDEX idx_tx_method ON monitored_transactions(decoded_method);
CREATE INDEX idx_tx_created_at ON monitored_transactions(created_at DESC);

-- Token Transfers Table (ERC20/ERC721/ERC1155)
CREATE TABLE token_transfers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tx_hash VARCHAR(66) NOT NULL REFERENCES monitored_transactions(tx_hash),
    log_index INT NOT NULL,
    block_number BIGINT NOT NULL,
    
    token_address VARCHAR(42) NOT NULL,
    token_type VARCHAR(20) NOT NULL,
    
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    
    amount NUMERIC(78, 0),
    token_id NUMERIC(78, 0),
    
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(tx_hash, log_index)
);

CREATE INDEX idx_token_tx_hash ON token_transfers(tx_hash);
CREATE INDEX idx_token_address ON token_transfers(token_address);
CREATE INDEX idx_token_from ON token_transfers(from_address);
CREATE INDEX idx_token_to ON token_transfers(to_address);
CREATE INDEX idx_token_timestamp ON token_transfers(timestamp DESC);

-- Events/Logs Table
CREATE TABLE contract_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tx_hash VARCHAR(66) NOT NULL REFERENCES monitored_transactions(tx_hash),
    log_index INT NOT NULL,
    block_number BIGINT NOT NULL,
    
    contract_address VARCHAR(42) NOT NULL,
    event_signature VARCHAR(66) NOT NULL,
    event_name VARCHAR(100),
    
    topics TEXT[],
    data TEXT,
    decoded_data JSONB,
    
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(tx_hash, log_index)
);

CREATE INDEX idx_events_tx_hash ON contract_events(tx_hash);
CREATE INDEX idx_events_contract ON contract_events(contract_address);
CREATE INDEX idx_events_signature ON contract_events(event_signature);
CREATE INDEX idx_events_name ON contract_events(event_name);
CREATE INDEX idx_events_timestamp ON contract_events(timestamp DESC);

-- =============================================================================
-- MONITORING & ALERTS
-- =============================================================================

-- Alerts Table (Enhanced)
CREATE TABLE alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    
    tx_hash VARCHAR(66),
    block_number BIGINT,
    address VARCHAR(42),
    
    metadata JSONB DEFAULT '{}',
    
    acknowledged BOOLEAN DEFAULT FALSE,
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by VARCHAR(100),
    
    email_sent BOOLEAN DEFAULT FALSE,
    telegram_sent BOOLEAN DEFAULT FALSE,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_alerts_timestamp ON alerts(created_at DESC);
CREATE INDEX idx_alerts_type ON alerts(alert_type);
CREATE INDEX idx_alerts_severity ON alerts(severity);
CREATE INDEX idx_alerts_acknowledged ON alerts(acknowledged);
CREATE INDEX idx_alerts_tx_hash ON alerts(tx_hash);

-- Address Watch List
CREATE TABLE watched_addresses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address VARCHAR(42) UNIQUE NOT NULL,
    label VARCHAR(100),
    address_type VARCHAR(50),
    watch_reason TEXT,
    alert_on_activity BOOLEAN DEFAULT TRUE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_watched_address ON watched_addresses(address);
CREATE INDEX idx_watched_type ON watched_addresses(address_type);

-- =============================================================================
-- ANALYTICS & STATS
-- =============================================================================

-- Daily Statistics
CREATE MATERIALIZED VIEW daily_stats AS
SELECT 
    DATE(block_timestamp) as date,
    COUNT(*) as tx_count,
    COUNT(*) FILTER (WHERE status = 'success') as success_count,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
    ROUND(COUNT(*) FILTER (WHERE status = 'failed')::NUMERIC / COUNT(*) * 100, 2) as failure_rate,
    
    SUM(value) as total_value_transferred,
    AVG(value) as avg_value,
    MAX(value) as max_value,
    
    AVG(gas_used) as avg_gas_used,
    SUM(gas_used * effective_gas_price) / 1e18 as total_gas_cost_eth,
    
    COUNT(DISTINCT from_address) as unique_senders,
    COUNT(DISTINCT to_address) as unique_receivers,
    COUNT(DISTINCT CASE WHEN tx_type LIKE '%contract%' THEN contract_address END) as unique_contracts
FROM monitored_transactions
GROUP BY DATE(block_timestamp)
ORDER BY date DESC;

CREATE UNIQUE INDEX idx_daily_stats_date ON daily_stats(date);

-- Hourly Statistics
CREATE MATERIALIZED VIEW hourly_stats AS
SELECT 
    date_trunc('hour', block_timestamp) as hour,
    COUNT(*) as tx_count,
    COUNT(*) FILTER (WHERE status = 'success') as success_count,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_count,
    AVG(gas_used) as avg_gas,
    SUM(value) as total_volume,
    COUNT(DISTINCT from_address) as unique_senders,
    AVG(effective_gas_price) as avg_gas_price
FROM monitored_transactions
GROUP BY date_trunc('hour', block_timestamp)
ORDER BY hour DESC;

CREATE UNIQUE INDEX idx_hourly_stats_hour ON hourly_stats(hour);

-- Top Addresses by Activity
CREATE MATERIALIZED VIEW top_addresses AS
SELECT 
    address,
    tx_count,
    total_value,
    first_seen,
    last_seen
FROM (
    SELECT 
        from_address as address,
        COUNT(*) as tx_count,
        SUM(value) as total_value,
        MIN(block_timestamp) as first_seen,
        MAX(block_timestamp) as last_seen
    FROM monitored_transactions
    GROUP BY from_address
    
    UNION ALL
    
    SELECT 
        to_address as address,
        COUNT(*) as tx_count,
        SUM(value) as total_value,
        MIN(block_timestamp) as first_seen,
        MAX(block_timestamp) as last_seen
    FROM monitored_transactions
    WHERE to_address IS NOT NULL
    GROUP BY to_address
) combined
GROUP BY address, tx_count, total_value, first_seen, last_seen
ORDER BY tx_count DESC
LIMIT 1000;

CREATE UNIQUE INDEX idx_top_addresses ON top_addresses(address);

-- =============================================================================
-- UTILITY FUNCTIONS
-- =============================================================================

-- Auto-update timestamp function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers
CREATE TRIGGER update_transactions_updated_at 
    BEFORE UPDATE ON monitored_transactions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_watched_addresses_updated_at 
    BEFORE UPDATE ON watched_addresses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Function to refresh materialized views
CREATE OR REPLACE FUNCTION refresh_stats_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY daily_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY hourly_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY top_addresses;
END;
$$ LANGUAGE plpgsql;

-- =============================================================================
-- INDEXER STATE TRACKING
-- =============================================================================

CREATE TABLE indexer_state (
    id SERIAL PRIMARY KEY,
    key VARCHAR(50) UNIQUE NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

INSERT INTO indexer_state (key, value) VALUES 
    ('last_indexed_block', '{"block_number": 0}'::jsonb),
    ('indexer_status', '{"status": "stopped", "started_at": null}'::jsonb);

\q
