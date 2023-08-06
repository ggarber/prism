use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;

use tokio::net::UdpSocket;
use tracing::*;
use webrtc_ice::{
    agent::{agent_config::AgentConfig, Agent},
    network_type::NetworkType,
    state::ConnectionState,
    udp_mux::{UDPMuxDefault, UDPMuxParams},
    udp_network::UDPNetwork,
};

use crate::transport;
use crate::{module, server};

pub mod rpc;

pub struct WebRtcModule {
    name: String,
    server: Arc<std::sync::Mutex<server::Server>>,
}

impl WebRtcModule {
    pub fn new(server: Arc<std::sync::Mutex<server::Server>>) -> Self {
        Self {
            name: "webrtc".to_string(),
            server,
        }
    }

    pub async fn start(self: WebRtcModule) -> anyhow::Result<()> {
        info!("webrtc start");

        let udp_socket = UdpSocket::bind("[::]:4435").await.unwrap();
        let udp_mux = UDPMuxDefault::new(UDPMuxParams::new(udp_socket));
        let udp_network = UDPNetwork::Muxed(udp_mux);

        let module = module::Module::new();
        let mut commands = module.commands.subscribe();
        self.server
            .lock()
            .unwrap()
            .modules
            .insert(self.name, Box::new(Arc::new(Mutex::new(module))));

        tokio::spawn(async move {
            let udp_network = udp_network.clone();
            while let Ok(msg) = commands.recv().await {
                info!("webrtc command: {:#?}", msg);

                let udp_network = udp_network.clone();
                // when ready to create a connection, create an agent
                let ice_agent = Arc::new(
                    Agent::new(AgentConfig {
                        network_types: vec![NetworkType::Udp4, NetworkType::Udp6],
                        udp_network,
                        ..Default::default()
                    })
                    .await
                    .unwrap(),
                );
                let _ = ice_agent.gather_candidates();

                // Get the local auth details and send to remote peer
                let (local_ufrag, local_pwd) = ice_agent.get_local_user_credentials().await;

                msg.reply.send(format!("{}/{}", local_ufrag, local_pwd)).unwrap();

                tokio::spawn(async move {
                    ice_agent.on_connection_state_change(Box::new(move |c: ConnectionState| {
                        info!("incoming connection ice");
                        if c == ConnectionState::Failed {
                            // let _ = ice_done_tx.try_send(());
                        }
                        Box::pin(async move {})
                    }));
                });
            }
        });

        Ok(())
    }

    pub async fn stop() -> anyhow::Result<()> {
        info!("webrtc stop");
        Ok(())
    }

    pub async fn exec(command: &str) -> anyhow::Result<()> {
        info!("webrtc exec {}", command);
        Ok(())
    }
}

pub struct WebRtcTransport {
    server: Arc<std::sync::Mutex<server::Server>>,
}

impl WebRtcTransport {
    pub fn new(server: Arc<std::sync::Mutex<server::Server>>) -> Self {
        Self { server }
    }
}

#[async_trait]
impl transport::Transport for WebRtcTransport {
    fn close(&self) {
        unimplemented!()
    }

    async fn process(self) -> Result<(), anyhow::Error> {
        info!("webrtc connection established");

        info!("webrtc connection finished");
        Ok(())
    }
}
