
class WebSocketConnection {
  get connected() {
    return this.ws && this.ws.readyState === WebSocket.OPEN;
  }

  async connect(url, closed) {
    this.ws = new WebSocket(url);
    this.ws.onclose = (event) => closed();
  }

  async send(data) {
    this.ws.send(data);
  }

  async read(callback) {
    this.ws.onmessage = (event) => {
      callback(event.data);
    };
  }

  async close() {
    if (!this.ws) {
      return;
    }
    this.ws.close();
    this.ws = null;
  }
}
