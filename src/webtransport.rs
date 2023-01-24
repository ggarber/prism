use std::sync::Arc;

use async_trait::async_trait;
use quinn::Connecting;

use futures_util::select;
use futures_util::FutureExt;

use tracing::*;

use bytes::Bytes;
use http::{Request, StatusCode};

use h3::{quic::BidiStream, server::RequestStream};

use crate::server;
use crate::transport;

pub struct WebTransport {
    server: Arc<std::sync::Mutex<server::Server>>,
    connecting: Connecting,
}

impl WebTransport {
    pub fn new(server: Arc<std::sync::Mutex<server::Server>>, connecting: Connecting) -> Self {
        Self { server, connecting }
    }
}

#[async_trait]
impl transport::Transport for WebTransport {
    fn close(&self) {
        unimplemented!()
    }

    async fn process(self) -> Result<(), anyhow::Error> {
        match self.connecting.await {
            Ok(conn) => {
                info!("connection established");

                let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
                    .await
                    .unwrap();

                let (channel, _stream) = match h3_conn.accept().await {
                    Ok(Some((req, mut stream))) => {
                        info!("connection new stream and request: {:#?}", req);

                        match handle_request(req, &mut stream).await {
                            Ok(channel_name) => {
                                // let transport = WebTransport::new();
                                let channel = self
                                    .server
                                    .lock()
                                    .unwrap()
                                    .find_or_create_channel(&channel_name);
                                (channel, stream)
                            }
                            Err(err) => {
                                error!("handling request failed: {}", err);
                                anyhow::bail!("invalid request")
                            }
                        }
                    }
                    Ok(None) => anyhow::bail!("no request stream"),
                    Err(err) => anyhow::bail!("invalid request {}", err),
                };

                let guard = channel.lock().await;
                let tx = guard.broadcast.clone();
                let mut rx = tx.subscribe();
                let channel_name = guard.name.clone();

                info!("connection request accepted: {:#?}", channel_name);
                drop(guard);

                loop {
                    select! {
                        data = h3_conn.poll_datagrams().fuse() => {
                            match data {
                                Ok(Some(datagram)) => {
                                    debug!("received: {:#?}", datagram.len());

                                    let _ = tx.send(datagram.into());
                                },
                                Ok(None) => {
                                    warn!("no more datagrams");
                                    break;
                                },
                                Err(err) => {
                                    error!("error on poll_datagrams {}", err);
                                    break;
                                }
                            }
                        },
                        res = rx.recv().fuse() => {
                            match res {
                                Ok(datagram) => {
                                    debug!("sent: {:#?}", datagram.len());

                                    let _ = h3_conn.send_datagram(datagram.into()).await;
                                },
                                Err(err) => {
                                    error!("no more datagrams {}", err);
                                    break;
                                }
                            }
                        }
                    }
                }

                // server.lock().unwrap().destroy_channel(&channel_name);
            }
            Err(err) => {
                error!("accepting connection failed: {:?}", err);
            }
        }

        info!("connection finished");
        Ok(())
    }
}

async fn handle_request<T>(
    req: Request<()>,
    stream: &mut RequestStream<T, Bytes>,
) -> Result<String, Box<dyn std::error::Error>>
where
    T: BidiStream<Bytes>,
{
    // Only accept webtransport requests
    if req.method() != "CONNECT" {
        // TODO: Check webtransport protocol
        return Err("invalid method".into());
    }

    let path = req.uri().path();
    let tokens: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if (tokens.len() != 2) || (tokens[0] != "channels") {
        return Err("invalid query".into());
    }
    let channel = tokens[1].to_owned();

    let resp = http::Response::builder()
        .status(StatusCode::OK)
        .header("sec-webtransport-http3-draft", "draft02")
        .body(())
        .unwrap();

    match stream.send_response(resp).await {
        Ok(_) => Ok(channel),
        Err(err) => Err(err.into()),
    }
}
