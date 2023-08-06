function Connect() {
  const data = new Uint8Array(30);
  const view = new DataView(data.buffer);
  view.setUint32(0, 0, false);
  view.setUint32(0+4, 30, false);
  view.setUint32(8, 0, false);
  view.setUint32(8+4, 0, false);
  view.setUint8(16, 0x00);  // Type: Connect
  view.setUint8(17, 0x00);  // Version: 0x00
  view.setUint16(18, 0x01);  // Audio Timescale
  view.setUint16(20, 0x01);  // Video Timescale
  view.setUint32(22, 0, false);
  view.setUint32(22+4, 0, false);
  return data;
}

if (!window.WebTransport) {
    log('WebTransport is not supported in this browser!!!');
}

class RushConnection {
  get connected() {
    return this.connectStream;
  }

  async connect(url, closed) {
    this.closed = closed;
    this.transport = new WebTransport(url);
    this.transport.closed.then(() => {
      this.closed();
      this.connectStream = null;
    }).catch((error) => {
      this.closed(error);
      this.writer = null;
    });

    await this.transport.ready;

    this.connectStream = await this.transport.createBidirectionalStream();

    this.writer = this.connectStream.writable.getWriter();
    this.writer.write(Connect());
  }

  async send(data) {
    if (!this.connectStream) {
      return;
    }
    await this.writer.write(data);
  }

  async read(callback) {
    if (!this.connectStream.readable) {
      return;
    }
    const reader = this.connectStream.readable.getReader();
    while (true) {
      const {value, done} = await reader.read();
      if (done) {
          break;
      }
      callback(value);
    }
  }

  async close() {
    if (!this.transport) {
      return;
    }
    try {
      await this.transport.close();
    } catch {
    } finally {
      this.transport = null;
    }
  }
}