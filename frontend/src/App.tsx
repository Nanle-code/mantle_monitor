import React, { useState, useEffect } from 'react';
import { Activity, AlertCircle, CheckCircle, Clock, TrendingUp, DollarSign, Users, Zap, Download, RefreshCw, Filter } from 'lucide-react';

// Type definitions
interface Transaction {
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

interface Alert {
  id: string;
  alert_type: string;
  severity: string;
  message: string;
  tx_hash?: string;
  metadata?: any;
  acknowledged: boolean;
  timestamp: string;
}

interface Stats {
  total_txs: number;
  success_rate: number;
  failed_rate: number;
  avg_gas_used: number;
  total_volume: string;
  active_users: number;
  tps: number;
}

const API_URL = 'http://localhost:3000';
const WS_URL = 'ws://localhost:3000/ws';

const TransactionMonitor = () => {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [stats, setStats] = useState<Stats>({
    total_txs: 0,
    success_rate: 0,
    failed_rate: 0,
    avg_gas_used: 0,
    total_volume: '0',
    active_users: 0,
    tps: 0
  });
  const [filter, setFilter] = useState<string>('all');
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [wsConnected, setWsConnected] = useState<boolean>(false);
  const [refreshing, setRefreshing] = useState<boolean>(false);

  // Load transactions based on filter
  const loadTransactions = async (status?: string) => {
    try {
      setRefreshing(true);
      const params = new URLSearchParams();
      params.append('limit', '50');
      if (status && status !== 'all') {
        params.append('status', status);
      }
      
      const response = await fetch(`${API_URL}/api/transactions?${params}`);
      if (!response.ok) throw new Error('Failed to fetch transactions');
      const data = await response.json();
      setTransactions(data);
      console.log(`Loaded ${data.length} transactions with filter: ${status || 'all'}`);
    } catch (error) {
      console.error('Failed to load transactions:', error);
    } finally {
      setRefreshing(false);
    }
  };

  const loadStats = async () => {
    try {
      const response = await fetch(`${API_URL}/api/stats`);
      if (!response.ok) throw new Error('Failed to fetch stats');
      const data = await response.json();
      setStats(data);
    } catch (error) {
      console.error('Failed to load stats:', error);
    }
  };

  const loadAlerts = async () => {
    try {
      const response = await fetch(`${API_URL}/api/alerts?limit=20`);
      if (!response.ok) throw new Error('Failed to fetch alerts');
      const data = await response.json();
      setAlerts(data);
    } catch (error) {
      console.error('Failed to load alerts:', error);
    }
  };

  // Load initial data
  useEffect(() => {
    const loadInitialData = async () => {
      setLoading(true);
      await Promise.all([
        loadTransactions(),
        loadStats(),
        loadAlerts()
      ]);
      setLoading(false);
    };

    loadInitialData();
    const interval = setInterval(loadStats, 10000);
    return () => clearInterval(interval);
  }, []);

  // Setup WebSocket
  useEffect(() => {
    const ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      console.log('WebSocket connected');
      setWsConnected(true);
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        
        if (message.type === 'transaction') {
          console.log('New transaction received:', message.data);
          setTransactions(prev => [message.data, ...prev.slice(0, 49)]);
          loadStats();
        } else if (message.type === 'alert') {
          console.log('New alert received:', message.data);
          setAlerts(prev => [message.data, ...prev.slice(0, 19)]);
        }
      } catch (error) {
        console.error('WebSocket message error:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      setWsConnected(false);
    };

    ws.onclose = () => {
      console.log('WebSocket disconnected');
      setWsConnected(false);
      setTimeout(() => {
        console.log('Reconnecting...');
      }, 5000);
    };

    return () => {
      ws.close();
    };
  }, []);

  // Handle filter change
  const handleFilterChange = async (newFilter: string) => {
    setFilter(newFilter);
    await loadTransactions(newFilter === 'all' ? undefined : newFilter);
  };

  const handleExport = async () => {
    try {
      const response = await fetch(`${API_URL}/api/export/csv`);
      const blob = await response.blob();
      const url = window.URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', `pharos_transactions_${Date.now()}.csv`);
      document.body.appendChild(link);
      link.click();
      link.remove();
    } catch (error) {
      console.error('Failed to export CSV:', error);
      alert('Failed to export CSV. Please try again.');
    }
  };

  const handleRefresh = () => {
    loadTransactions(filter === 'all' ? undefined : filter);
    loadStats();
    loadAlerts();
  };

  const getStatusColor = (status: string) => {
    if (!status) return 'text-gray-600 bg-gray-100 border-gray-300';
    switch(status.toLowerCase()) {
      case 'success': return 'text-emerald-700 bg-emerald-50 border-emerald-200';
      case 'pending': return 'text-amber-700 bg-amber-50 border-amber-200';
      case 'failed': return 'text-rose-700 bg-rose-50 border-rose-200';
      default: return 'text-gray-600 bg-gray-100 border-gray-300';
    }
  };

  const getStatusIcon = (status: string) => {
    if (!status) return null;
    switch(status.toLowerCase()) {
      case 'success': return <CheckCircle className="w-4 h-4" />;
      case 'pending': return <Clock className="w-4 h-4" />;
      case 'failed': return <AlertCircle className="w-4 h-4" />;
      default: return null;
    }
  };

  const formatAmount = (amount: string) => {
    const num = parseFloat(amount || '0');
    if (num === 0) return '0.00';
    if (num < 0.01) return num.toExponential(2);
    return num.toFixed(2);
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-900 via-indigo-900 to-purple-900 flex items-center justify-center">
        <div className="text-center">
          <Activity className="w-16 h-16 text-indigo-400 animate-spin mx-auto mb-4" />
          <div className="text-white text-2xl font-semibold">Loading Mantle Monitor...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-indigo-900 to-purple-900 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-8 bg-white/10 backdrop-blur-xl rounded-2xl p-6 border border-white/20 shadow-2xl">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="bg-gradient-to-br from-indigo-500 to-purple-600 p-3 rounded-xl shadow-lg">
                <Activity className="w-8 h-8 text-white" />
              </div>
              <div>
                <h1 className="text-3xl font-bold text-white mb-1">Mantle Transaction Monitor</h1>
                <p className="text-indigo-200">Real-time blockchain monitoring system</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <div className={`flex items-center gap-2 px-4 py-2 rounded-xl font-medium transition-all ${
                wsConnected 
                  ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-400/30' 
                  : 'bg-rose-500/20 text-rose-300 border border-rose-400/30'
              }`}>
                <div className={`w-2 h-2 rounded-full ${wsConnected ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'}`}></div>
                <span className="text-sm">{wsConnected ? 'Connected' : 'Disconnected'}</span>
              </div>
              <button
                onClick={handleRefresh}
                disabled={refreshing}
                className="flex items-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:bg-indigo-800 text-white rounded-xl transition-all shadow-lg hover:shadow-xl disabled:cursor-not-allowed"
              >
                <RefreshCw className={`w-4 h-4 ${refreshing ? 'animate-spin' : ''}`} />
                Refresh
              </button>
              <button
                onClick={handleExport}
                className="flex items-center gap-2 px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-xl transition-all shadow-lg hover:shadow-xl"
              >
                <Download className="w-4 h-4" />
                Export CSV
              </button>
            </div>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-6">
          {[
            { icon: Activity, label: 'Total TXs', value: stats.total_txs || 0, color: 'blue' },
            { icon: CheckCircle, label: 'Success Rate', value: `${stats.success_rate?.toFixed(1) || 0}%`, color: 'emerald' },
            { icon: Zap, label: 'Avg Gas', value: Math.floor(stats.avg_gas_used) || 0, color: 'amber' },
            { icon: DollarSign, label: 'Volume', value: `$${formatAmount(stats.total_volume)}`, color: 'purple' },
            { icon: Users, label: 'Active Users', value: stats.active_users || 0, color: 'pink' },
            { icon: TrendingUp, label: 'TPS', value: stats.tps?.toFixed(1) || 0, color: 'cyan' }
          ].map((stat, idx) => (
            <div key={idx} className="bg-white/10 backdrop-blur-xl rounded-xl p-4 border border-white/20 shadow-xl hover:shadow-2xl transition-all hover:scale-105">
              <div className="flex items-center gap-2 mb-2">
                <stat.icon className={`w-5 h-5 text-${stat.color}-400`} />
                <span className="text-gray-300 text-sm font-medium">{stat.label}</span>
              </div>
              <p className="text-2xl font-bold text-white">{stat.value}</p>
            </div>
          ))}
        </div>

        {/* Alerts Section */}
        {alerts.length > 0 && (
          <div className="bg-rose-900/30 backdrop-blur-xl rounded-xl p-5 border border-rose-500/40 mb-6 shadow-xl">
            <h3 className="text-white font-semibold mb-3 flex items-center gap-2 text-lg">
              <AlertCircle className="w-5 h-5 text-rose-400" />
              Recent Alerts
            </h3>
            <div className="space-y-2">
              {alerts.slice(0, 5).map(alert => (
                <div key={alert.id} className="flex items-start gap-3 text-sm bg-white/5 p-3 rounded-lg">
                  <span className="text-gray-400 font-mono text-xs">{new Date(alert.timestamp).toLocaleTimeString()}</span>
                  <span className={`flex-1 ${alert.severity === 'error' ? 'text-rose-300' : 'text-amber-300'}`}>
                    {alert.message}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Filters */}
        <div className="mb-6 bg-white/10 backdrop-blur-xl rounded-xl p-4 border border-white/20 shadow-xl">
          <div className="flex items-center gap-3">
            <Filter className="w-5 h-5 text-indigo-300" />
            <span className="text-white font-semibold">Filter by Status:</span>
            <div className="flex gap-2 flex-1">
              {[
                { value: 'all', label: 'All Transactions', color: 'indigo' },
                { value: 'success', label: 'Success', color: 'emerald' },
                { value: 'pending', label: 'Pending', color: 'amber' },
                { value: 'failed', label: 'Failed', color: 'rose' }
              ].map(f => (
                <button
                  key={f.value}
                  onClick={() => handleFilterChange(f.value)}
                  className={`px-4 py-2 rounded-lg font-medium transition-all ${
                    filter === f.value 
                      ? `bg-${f.color}-600 text-white shadow-lg scale-105` 
                      : 'bg-white/10 text-gray-300 hover:bg-white/20'
                  }`}
                >
                  {f.label}
                </button>
              ))}
            </div>
            <span className="text-indigo-300 text-sm font-medium">
              {transactions.length} transaction{transactions.length !== 1 ? 's' : ''}
            </span>
          </div>
        </div>

        {/* Transactions Table */}
        <div className="bg-white/10 backdrop-blur-xl rounded-xl border border-white/20 shadow-2xl overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-white/5">
                <tr className="border-b border-white/10">
                  <th className="text-left p-4 text-indigo-300 font-semibold">TX Hash</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">Type</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">Status</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">Amount</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">Gas</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">From</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">Block</th>
                  <th className="text-left p-4 text-indigo-300 font-semibold">Time</th>
                </tr>
              </thead>
              <tbody>
                {transactions.map((tx, index) => (
                  <tr key={tx.id || tx.tx_hash || index} className="border-b border-white/5 hover:bg-white/10 transition-colors">
                    <td className="p-4">
                      <code className="text-purple-300 text-sm font-mono bg-purple-900/30 px-2 py-1 rounded">
                        {(tx.tx_hash || '').slice(0, 12)}...
                      </code>
                    </td>
                    <td className="p-4">
                      <span className="inline-block px-3 py-1 rounded-lg bg-blue-500/30 text-blue-200 text-sm font-medium border border-blue-400/30">
                        {tx.tx_type || 'unknown'}
                      </span>
                    </td>
                    <td className="p-4">
                      <span className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-sm font-medium border ${getStatusColor(tx.status)}`}>
                        {getStatusIcon(tx.status)}
                        {tx.status || 'pending'}
                      </span>
                    </td>
                    <td className="p-4 text-white font-semibold">
                      ${formatAmount(tx.amount)}
                    </td>
                    <td className="p-4 text-gray-300 font-mono text-sm">{(tx.gas_used || 0).toLocaleString()}</td>
                    <td className="p-4">
                      <code className="text-gray-400 text-xs font-mono bg-gray-800/50 px-2 py-1 rounded">
                        {(tx.from_address || '').slice(0, 10)}...
                      </code>
                    </td>
                    <td className="p-4 text-gray-300 font-mono text-sm">{(tx.block_number || 0).toLocaleString()}</td>
                    <td className="p-4 text-gray-400 text-sm">
                      {tx.block_timestamp ? new Date(tx.block_timestamp).toLocaleTimeString() : 'N/A'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {transactions.length === 0 && (
          <div className="text-center py-16 bg-white/5 backdrop-blur-xl rounded-xl mt-6 border border-white/10">
            <Filter className="w-16 h-16 text-gray-500 mx-auto mb-4" />
            <p className="text-gray-400 text-lg">No transactions found for this filter</p>
            <p className="text-gray-500 text-sm mt-2">Try selecting a different status filter</p>
          </div>
        )}

        {/* Footer */}
        <div className="mt-8 text-center text-gray-400 text-sm">
          <p>Monitoring Mantle Blockchain</p>
        </div>
      </div>
    </div>
  );
};

export default TransactionMonitor;