use std::{
    sync::{Arc, Mutex}, collections::HashMap,
};

use crate::channel;

#[derive(Debug)]
pub struct Server {
    channels: HashMap<String, Arc<Mutex<channel::Channel>>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn find_or_create_channel(&mut self, name: &str) -> Arc<Mutex<channel::Channel>> {
        let channel = self.channels.get(name);
        match channel {
            Some(channel) => channel.clone(),
            None => {
                let channel = Arc::new(Mutex::new(channel::Channel::new(name))) ;
                self.channels.insert(name.to_string(), channel.clone());
                channel.clone()
            }
        }
    }
}
