use std::{sync::Arc, fs, time::Duration, path::PathBuf, task::Poll};
use anyhow::Context;
use quinn::Connecting;
use tokio::sync::Mutex;
use futures_util::{future, FutureExt, StreamExt, select};

use bytes::{Bytes};
use http::{Request, StatusCode};

use h3::{quic::{BidiStream, RecvStream, SendStream}, server::{RequestStream, Frame}};
use h3_quinn::quinn;
use tracing::*;

use crate::{server, module, rush::messages::{parse, RushMessages}};

pub mod messages;

const ALPN_RUSH: &[&[u8]] = &[b"h3"];

pub struct RushModule {
    name: String,
    server: server::ServerPtr,
}

impl RushModule {
    pub fn new(
        server: server::ServerPtr,
    ) -> Self {
        Self { name: "rush".to_string(), server }
    }

    pub async fn start(self: RushModule)-> anyhow::Result<()> {
        info!("rush start");

        let module = module::Module::new();
        self.server.lock()
            .unwrap().modules.insert(
                self.name, 
                Box::new(Arc::new(Mutex::new(module)))
            );

        let key_file = PathBuf::from("ssl.key");
        let cert_file = PathBuf::from("ssl.crt");
        let key = fs::read(key_file.clone()).context("failed to read private key")?;
        let key = if key_file.extension().map_or(false, |x| x == "der") {
            rustls::PrivateKey(key)
        } else {
            let pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut &*key)
                .context("malformed PKCS #8 private key")?;
            match pkcs8.into_iter().next() {
                Some(x) => rustls::PrivateKey(x),
                None => {
                    let rsa = rustls_pemfile::rsa_private_keys(&mut &*key)
                        .context("malformed PKCS #1 private key")?;
                    match rsa.into_iter().next() {
                        Some(x) => rustls::PrivateKey(x),
                        None => {
                            anyhow::bail!("no private keys found");
                        }
                    }
                }
            }
        };
        let certs = fs::read(cert_file.clone()).context("failed to read certificate chain")?;
        let certs = if cert_file.extension().map_or(false, |x| x == "der") {
            vec![rustls::Certificate(certs)]
        } else {
            rustls_pemfile::certs(&mut &*certs)
                .context("invalid PEM-encoded certificate")?
                .into_iter()
                .map(rustls::Certificate)
                .collect()
        };
    
        let mut wt_tls = rustls::ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certs.clone(), key.clone())?;
    
        wt_tls.max_early_data_size = u32::MAX; // TODO
        wt_tls.alpn_protocols = ALPN_RUSH.iter().map(|&x| x.into()).collect();
        wt_tls.key_log = Arc::new(rustls::KeyLogFile::new());
        
        let mut config = quinn::ServerConfig::with_crypto(Arc::new(wt_tls));
        let transport_config = Arc::get_mut(&mut config.transport).unwrap();
        transport_config.max_idle_timeout(Some(Duration::from_secs(600).try_into().unwrap()));
    
        let (endpoint, mut incoming) = quinn::Endpoint::server(config, "[::]:3446".parse().unwrap())?;
        info!("listening rush on {}", endpoint.local_addr()?);
        
        tokio::spawn(async move {
            while let Some(new_conn) = incoming.next().await {
                trace_span!("New connection being attempted");
        
                let server = self.server.clone();
                tokio::spawn(async move {
                    let _ = process(server, new_conn).await;
                });
            }
        });

        Ok(())
    }

    pub async fn stop() -> anyhow::Result<()> {
        info!("rush stop");

        // endpoint.wait_idle().await;

        Ok(())
    }

    pub async fn exec(command: &str) -> anyhow::Result<()> {
        info!("rush exec {}", command);
        Ok(())
    }
}


async fn process(server: server::ServerPtr, new_conn: Connecting) -> anyhow::Result<()> {
    match new_conn.await {
        Ok(conn) => {
            info!("new connection established");
            
            let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
                .await
                .unwrap();

            let (channel, control_stream) = match h3_conn.accept().await {
                Ok(Some((req, mut stream))) => {
                    info!("new request: {:#?}", req);

                    match handle_request(req, &mut stream).await {
                        Ok(channel_name) => {
                            let channel = server
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
                },

                // indicating no more streams to be received
                Ok(None) => anyhow::bail!("no request stream"),

                Err(err) => anyhow::bail!("error on accept {}", err),
            };
            
            debug!("wait for streams");
            loop {
                match h3_conn.poll_accept_bidi().await {
                    Ok(Some(mut stream)) => {
                        info!("new stream");
                        
                        let guard = channel.lock().await;
                        let tx = guard.broadcast.clone();
                        let mut rx = tx.subscribe();
        
                        drop(guard);
        
                        // Buffer to store partial messages until they can be parsed
                        let mut buffer: Vec<u8> = Vec::new();
                        loop {
                            select! {
                                data = future::poll_fn(|cx| {
                                    // if let Poll::Ready(x) = stream.poll_ready(cx) {
                                    //     return Poll::Ready(Ok(None));
                                    // }
                                    stream.poll_data(cx)
                                }).fuse() => {
                                    match data {
                                        Ok(Some(data)) => {
                                            trace!("new data: {:?} buffer: {:?}", data.len(), buffer.len());
                                            // Ignore first one
                                            if data[0] == 0x40 {
                                                continue;
                                            }

                                            buffer.extend_from_slice(&data);
                                            
                                            match parse(&buffer) {
                                                Ok((msg, bytes)) => {
                                                    let processed = buffer.drain(0..bytes);
                                                    match msg {
                                                        RushMessages::Connect(msg) => {
                                                            debug!("new message: {:?}", msg);
                                                        }
                                                        RushMessages::AudioFrame(msg) => {
                                                            debug!("new audio message: {:?}", msg);
                                                            let _ = tx.send(processed.collect());
                                                        }
                                                        RushMessages::VideoFrame(msg) => {
                                                            debug!("new video message: {:?}", msg);
                                                            let _ = tx.send(processed.collect());
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    // info!("error on poll_data {}", err);
                                                }
                                            }
                                        }

                                        Ok(None) => {
                                            // anyhow::bail!("no more poll_data");
                                        }

                                        Err(err) => {
                                            anyhow::bail!("error on poll_data {}", err);
                                        }
                                    }
                                },

                                res = rx.recv().fuse() => {
                                    match res {
                                        Ok(message) => {
                                            debug!("sent: {:#?}", message.len());

                                            // TODO: poll_ready
        
                                            // let (mut send, mut recv) = stream.split();
                                            // let mut buf = BytesMut::with_capacity(4096 * 10);
                                            // let bytes: Bytes = message.into();
                                            let _ = stream.send_data(Frame::Data(message.into())); // (StreamType::ENCODER, Frame::Data(Bytes::from("hey"))));
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

                    Ok(None) => {
                        // return;
                    }

                    Err(err) => {
                        anyhow::bail!("error on wait {}", err);
                    }
                }
            }
        }
        Err(err) => {
            anyhow::bail!("accepting connection failed: {:?}", err);
        }
    };
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
