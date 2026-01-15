import axios from 'axios';

const API_URL = process.env.REACT_APP_API_URL || 'http://localhost:3000';

export const api = axios.create({
  baseURL: API_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

export const getTransactions = async (params?: {
  status?: string;
  tx_type?: string;
  limit?: number;
  offset?: number;
}) => {
  const response = await api.get('/api/transactions', { params });
  return response.data;
};

export const getStats = async () => {
  const response = await api.get('/api/stats');
  return response.data;
};

export const getAlerts = async (limit = 50) => {
  const response = await api.get('/api/alerts', {
    params: { limit },
  });
  return response.data;
};

export const exportCSV = async () => {
  const response = await api.get('/api/export/csv', {
    responseType: 'blob',
  });
  
  const url = window.URL.createObjectURL(new Blob([response.data]));
  const link = document.createElement('a');
  link.href = url;
  link.setAttribute('download', `pharos_transactions_${Date.now()}.csv`);
  document.body.appendChild(link);
  link.click();
  link.remove();
};