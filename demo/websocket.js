
class WebSocketConnection {
  get connected() {
    return this.ws && this.ws.readyState === WebSocket.OPEN;
  }

  async connect(url, closed) {
    this.ws = new WebSocket(url);
    this.ws.binaryType = 'arraybuffer';
    await new Promise((resolve, reject) => {
      this.ws.onclose = reject;
      this.ws.onerror = reject;
      this.ws.onopen = resolve;
    });
    this.ws.onclose = (event) => closed();
  }

  async send(data) {
    this.ws.send(data);
  }

  async read(callback) {
    this.ws.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) {
        callback(new Uint8Array(event.data));
      } else {
        // text frame
        console.log(event.data);
      }
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
