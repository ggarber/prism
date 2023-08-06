use hyper::service::{make_service_fn, service_fn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use tokio::sync::broadcast::{self};

use http::{Response};
use hyper::{server::Server, Body, Error};

use tracing::*;

use crate::{module, server};

pub struct WhipModule {
    name: String,
    server: server::ServerPtr,
}

impl WhipModule {
    pub fn new(server: server::ServerPtr) -> Self {
        Self {
            name: "whip".to_string(),
            server,
        }
    }

    pub async fn start(self: WhipModule) -> anyhow::Result<()> {
        info!("whip start");

        let webrtc = self
            .server
            .lock()
            .unwrap()
            .modules
            .get("webrtc")
            .unwrap()
            .clone();
        let guard = webrtc.lock().await;
        let commands = guard.commands.clone();

        let state = Arc::new(Mutex::new(commands));

        let service = make_service_fn(move |_| {
            let state = state.clone();
            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let state = state.clone();
                    async move {
                        if req.uri().path() != "/" {
                            return Ok::<_, Error>(Response::new(Body::from("Hello World")));
                        }

                        let (reply, _) = broadcast::channel::<String>(1);
                        let _ = state.lock().await.send(module::Message {
                            data: "Hi".to_string(),
                            reply: reply.clone(),
                        });
                        let res = reply.subscribe().recv().await.unwrap();
                        Ok::<_, Error>(Response::new(Body::from(res)))
                    }
                }))
            }
        });

        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let server = Server::bind(&addr)
            .serve(service);

        tokio::spawn(async move {
            server.await.expect("server error");
        });
        Ok(())
    }

    pub async fn stop() -> anyhow::Result<()> {
        info!("whip stop");
        Ok(())
    }

    pub async fn exec(command: &str) -> anyhow::Result<()> {
        info!("whip exec {}", command);
        Ok(())
    }
}
