use std::sync::Arc;

use async_trait::async_trait;

use futures_util::future;
use futures_util::select;
use futures_util::FutureExt;
use futures_util::SinkExt;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use http::Uri;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::Message;
use tracing::*;

use crate::server;
use crate::transport;
use crate::util;

pub struct WebSocket {
    server: Arc<std::sync::Mutex<server::Server>>,
    stream: TlsStream<TcpStream>,
}

impl WebSocket {
    pub fn new(
        server: Arc<std::sync::Mutex<server::Server>>,
        stream: TlsStream<TcpStream>,
    ) -> Self {
        Self { server, stream }
    }
}

#[async_trait]
impl transport::Transport for WebSocket {
    fn close(&self) {
        unimplemented!()
    }

    async fn process(self) -> Result<(), anyhow::Error> {
        info!("connection established");

        let mut uri: Uri = Default::default();
        let ws_stream = tokio_tungstenite::accept_hdr_async(self.stream, |req: &Request, res| {
            uri = req.uri().clone();
            Ok(res)
        })
        .await;

        match ws_stream {
            Ok(ws_stream) => {
                debug!("connection websocket handshaked {:?}", uri);

                let channel_name =
                    util::parse_channel(uri.path()).expect("invalid path, no channel found");
                let channel = self
                    .server
                    .lock()
                    .unwrap()
                    .find_or_create_channel(&channel_name);

                let guard = channel.lock().await;
                let tx = guard.broadcast.clone();
                let mut rx = tx.subscribe();
                let channel_name = guard.name.clone();

                info!("connection request accepted: {:#?}", channel_name);
                drop(guard);

                let (mut write, read) = ws_stream.split();
                let mut ws_read = read.try_filter(|msg| future::ready(msg.is_binary()));
                loop {
                    select! {
                        data = ws_read.next().fuse() => {
                            match data {
                                Some(Ok(Message::Binary(datagram))) => {
                                    debug!("received: {:#?}", datagram.len());

                                    let _ = tx.send(datagram.into());
                                },
                                Some(Err(err)) => {
                                    error!("error on poll_datagrams {}", err);
                                    break;
                                }
                                _ => {
                                    warn!("no more datagrams");
                                    break;
                                }
                            }
                        },
                        res = rx.recv().fuse() => {
                            match res {
                                Ok(datagram) => {
                                    debug!("sent: {:#?}", datagram.len());

                                    let _ = write.send(Message::Binary(datagram)).await;
                                },
                                Err(err) => {
                                    error!("no more datagrams {}", err);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                error!("connection websocket handshaked failed: {}", err);
            }
        }

        info!("connection finished");
        Ok(())
    }
}
