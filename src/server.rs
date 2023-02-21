use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::*;

use crate::channel;
use crate::module;

#[derive(Debug)]
pub struct Server {
    channels: HashMap<String, Arc<Mutex<channel::Channel>>>,
    pub modules: HashMap<String, Arc<Mutex<module::Module>>>,
}

pub type ServerPtr = Arc<std::sync::Mutex<Server>>;

impl Server {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    pub fn find_or_create_channel(&mut self, name: &str) -> Arc<Mutex<channel::Channel>> {
        let channel = self.channels.get(name);
        match channel {
            Some(channel) => channel.clone(),
            None => {
                info!("channel created {}", name);
                let channel = Arc::new(Mutex::new(channel::Channel::new(name)));
                self.channels.insert(name.to_string(), channel.clone());
                channel
            }
        }
    }

    // pub fn destroy_channel(&mut self, name: &str) -> void {
    //     let channel = self.channels.delete(name);
    // }
}
