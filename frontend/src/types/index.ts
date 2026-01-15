export interface Transaction {
  id: string;
  tx_hash: string;
  tx_type: string;
  status: string;
  from_address: string;
  to_address: string;
  amount: string;
  gas_used: number;
  gas_price: number;
  block_number: number;
  block_timestamp: string;
  created_at: string;
  updated_at: string;
}

export interface Alert {
  id: string;
  alert_type: string;
  severity: string;
  message: string;
  tx_hash?: string;
  metadata?: any;
  acknowledged: boolean;
  timestamp: string;
}

export interface Stats {
  total_txs: number;
  success_rate: number;
  failed_rate: number;
  avg_gas_used: number;
  total_volume: string;
  active_users: number;
  tps: number;
}