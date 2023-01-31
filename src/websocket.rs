use std::sync::Arc;

use async_trait::async_trait;

use futures_util::future;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use tokio::net::TcpStream;
use tracing::*;

use crate::server;
use crate::transport;

pub struct WebSocket {
    server: Arc<std::sync::Mutex<server::Server>>,
    stream: TcpStream,
}

impl WebSocket {
    pub fn new(server: Arc<std::sync::Mutex<server::Server>>, stream: TcpStream) -> Self {
        Self { server, stream }
    }
}

#[async_trait]
impl transport::Transport for WebSocket {
    fn close(&self) {
        unimplemented!()
    }

    async fn process(self) -> Result<(), anyhow::Error> {
        let addr = self
            .stream
            .peer_addr()
            .expect("connected streams should have a peer address");

        info!("connection established {}", addr);

        let ws_stream = tokio_tungstenite::accept_async(self.stream).await;

        match ws_stream {
            Ok(ws_stream) => {
                debug!("connection websocket handshaked");

                let (write, read) = ws_stream.split();
                // We should not forward messages other than text or binary.
                read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
                    .forward(write)
                    .await
                    .expect("failed to forward messages");
            }
            Err(err) => {
                error!("connection websocket handshaked failed: {}", err);
            }
        }

        info!("connection finished");
        Ok(())
    }
}
