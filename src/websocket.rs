use std::sync::Arc;

use async_trait::async_trait;

use tracing::*;

use crate::server;
use crate::transport;

pub struct WebSocket {
    server: Arc<std::sync::Mutex<server::Server>>,
}

impl WebSocket {
    pub fn new(server: Arc<std::sync::Mutex<server::Server>>) -> Self {
        Self { server }
    }
}

#[async_trait]
impl transport::Transport for WebSocket {
    fn close(&self) {
        unimplemented!()
    }

    async fn process(self) -> Result<(), anyhow::Error> {
        info!("connection established");

        info!("connection finished");
        Ok(())
    }
}
