class WebSocketService {
  private ws: WebSocket | null = null;
  private reconnectInterval = 5000;
  private url: string;
  private listeners: {
    transaction: Array<(data: any) => void>;
    alert: Array<(data: any) => void>;
    error: Array<(error: Event) => void>;
  };

  constructor(url: string) {
    this.url = url;
    this.listeners = {
      transaction: [],
      alert: [],
      error: [],
    };
  }

  connect() {
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log('✅ WebSocket connected');
    };

    this.ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        
        if (message.type === 'transaction') {
          this.listeners.transaction.forEach(callback => callback(message.data));
        } else if (message.type === 'alert') {
          this.listeners.alert.forEach(callback => callback(message.data));
        }
      } catch (error) {
        console.error('WebSocket message error:', error);
      }
    };

    this.ws.onerror = (error) => {
      console.error('❌ WebSocket error:', error);
      this.listeners.error.forEach(callback => callback(error));
    };

    this.ws.onclose = () => {
      console.log('🔌 WebSocket disconnected. Reconnecting...');
      setTimeout(() => {
        this.connect();
      }, this.reconnectInterval);
    };
  }

  on(event: 'transaction' | 'alert' | 'error', callback: (data: any) => void) {
    if (this.listeners[event]) {
      this.listeners[event].push(callback);
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}

export default WebSocketService;