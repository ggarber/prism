if (!window.WebTransport) {
  log('WebTransport is not supported in this browser!!!');
}

class WebTransportConnection {
  get connected() {
    return this.writer;
  }

  async connect(url, closed) {
    this.closed = closed;
    this.transport = new WebTransport(url);
    this.transport.closed.then(() => {
      this.closed();
      this.writer = null;
    }).catch((error) => {
      this.closed(error);
      this.writer = null;
    });

    await this.transport.ready;

    this.writer = this.transport.datagrams.writable.getWriter();
  }

  async send(data) {
    if (!this.writer) {
      return;
    }
    await this.writer.write(data);
  }

  async read(callback) {
    const reader = this.transport.datagrams.readable.getReader();
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
