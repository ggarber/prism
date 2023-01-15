use async_std::channel::{unbounded, Receiver, Sender};

#[derive(Debug)]
pub struct Channel {
    pub name: String,
    pub channels: (Sender<Vec<u8>>, Receiver<Vec<u8>>),
}

impl Channel {
    pub fn new(name: &str) -> Self {
        let (tx, rx) = unbounded();
        Self {
            name: name.to_string(),
            channels: (tx, rx),
        }
    }
}
