use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod channel;
mod server;

use anyhow::{Context, Result};
use bytes::Bytes;
use clap::Parser;
use futures_util::FutureExt;
use futures_util::{select, StreamExt};
use http::{Request, StatusCode};
use tracing::*;

use h3::{quic::BidiStream, server::RequestStream};
use h3_quinn::quinn;

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"h3"];

#[derive(Parser, Debug)]
#[clap(name = "prism")]
struct Opt {
    /// TLS private key in PEM format
    #[clap(short = 'k', long = "key", requires = "cert")]
    key: PathBuf,
    /// TLS certificate in PEM format
    #[clap(short = 'c', long = "cert", requires = "key")]
    cert: PathBuf,
    /// Address to listen on
    #[clap(long = "listen", default_value = "[::1]:4433")]
    listen: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
    .unwrap();

    let options = Opt::parse();

    let key = fs::read(options.key.clone()).context("failed to read private key")?;
    let key = if options.key.extension().map_or(false, |x| x == "der") {
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
    let certs = fs::read(options.cert.clone()).context("failed to read certificate chain")?;
    let certs = if options.cert.extension().map_or(false, |x| x == "der") {
        vec![rustls::Certificate(certs)]
    } else {
        rustls_pemfile::certs(&mut &*certs)
            .context("invalid PEM-encoded certificate")?
            .into_iter()
            .map(rustls::Certificate)
            .collect()
    };

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    tls_config.max_early_data_size = u32::MAX;
    tls_config.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    tls_config.key_log = Arc::new(rustls::KeyLogFile::new());

    let config = quinn::ServerConfig::with_crypto(Arc::new(tls_config));

    let (endpoint, mut incoming) = quinn::Endpoint::server(config, options.listen)?;
    info!("listening on {}", endpoint.local_addr()?);

    let server = Arc::new(Mutex::new(server::Server::new()));

    while let Some(new_conn) = incoming.next().await {
        info!("new incoming connection");

        let server = server.clone();
        tokio::spawn(async move {
            match new_conn.await {
                Ok(conn) => {
                    info!("new connection established");

                    let mut h3_conn = h3::server::Connection::new(h3_quinn::Connection::new(conn))
                        .await
                        .unwrap();

                    let (channel, _stream) = match h3_conn.accept().await {
                        Ok(Some((req, mut stream))) => {
                            info!("new stream and request: {:#?}", req);

                            match handle_request(req, &mut stream).await {
                                Ok(channel_name) => {
                                    // let transport = WebTransport::new();
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
                        }
                        Ok(None) => anyhow::bail!("no request stream"),
                        Err(err) => anyhow::bail!("invalid request {}", err),
                    };

                    info!("request accepted: {:#?}", channel);

                    let guard = channel.lock().await;
                    let (send, recv) = &guard.channels;

                    loop {
                        select! {
                            data = h3_conn.poll_datagrams().fuse() => {
                                match data {
                                    Ok(Some(datagram)) => {
                                        debug!("received: {:#?}", datagram.len());
                                        // Avoid awaiting here if possible
                                        let _ = send.send(datagram.into()).await;
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
                            res = recv.recv().fuse() => {
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
                }
                Err(err) => {
                    error!("accepting connection failed: {:?}", err);
                }
            }
            Ok(())
        });
    }

    Ok(())
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
        return Err("invalid method".into());
    }

    let tokens: Vec<&str> = req
        .uri()
        .path()
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
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
