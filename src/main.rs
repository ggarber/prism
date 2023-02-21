use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::*;

use h3_quinn::quinn;

pub mod channel;
pub mod module;
pub mod server;
pub mod transport;
pub mod util;
pub mod webrtc;
pub mod websocket;
pub mod webtransport;
pub mod whip;
use crate::transport::Transport;

const ALPN_QUIC_HTTP: &[&[u8]] = &[b"h3", b"rush"];

#[derive(Parser, Debug)]
#[clap(name = "prism")]
struct Opt {
    /// TLS private key in PEM format
    #[clap(short = 'k', long = "key", requires = "cert")]
    key: PathBuf,
    /// TLS certificate in PEM format
    #[clap(short = 'c', long = "cert", requires = "key")]
    cert: PathBuf,
    /// Address to listen on for quic
    #[clap(long = "wt_listen", default_value = "[::]:4433")]
    wt_listen: SocketAddr,
    /// Address to listen on for quic
    #[clap(long = "ws_listen", default_value = "[::]:4434")]
    ws_listen: SocketAddr,
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

    let mut wt_tls = rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs.clone(), key.clone())?;

    wt_tls.max_early_data_size = u32::MAX; // TODO
    wt_tls.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    wt_tls.key_log = Arc::new(rustls::KeyLogFile::new());

    let mut config = quinn::ServerConfig::with_crypto(Arc::new(wt_tls));
    let transport_config = Arc::get_mut(&mut config.transport).unwrap();
    transport_config.max_idle_timeout(Some(Duration::from_secs(600).try_into().unwrap()));

    let (endpoint, mut incoming) = quinn::Endpoint::server(config, options.wt_listen)?;
    info!("listening webtransport on {}", endpoint.local_addr()?);

    let server = Arc::new(Mutex::new(server::Server::new()));

    let webrtc = webrtc::WebRtcModule::new(server.clone());
    webrtc.start().await?;

    let whip = whip::WhipModule::new(server.clone());
    whip.start().await?;

    let clone = server.clone();
    tokio::spawn(async move {
        while let Some(new_conn) = incoming.next().await {
            info!("incoming connection quic");

            let server = clone.clone();
            tokio::spawn(async move {
                let transport = webtransport::WebTransport::new(server, new_conn);
                let _ = transport.process().await;
            });
        }
    });

    let mut ws_tls = rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs.clone(), key.clone())?;
    ws_tls.max_early_data_size = u32::MAX; // TODO
    ws_tls.key_log = Arc::new(rustls::KeyLogFile::new());

    let acceptor = TlsAcceptor::from(Arc::new(ws_tls));
    let listener = TcpListener::bind(options.ws_listen).await?;
    info!("listening websocket on {}", listener.local_addr()?);

    while let Ok((stream, _)) = listener.accept().await {
        info!("incoming connection tcp");

        let server = server.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let stream = acceptor.accept(stream).await;
            let transport = websocket::WebSocket::new(server, stream.unwrap());
            let _ = transport.process().await;
        });
    }

    Ok(())
}
