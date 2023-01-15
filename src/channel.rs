use async_std::channel::{ unbounded, Sender, Receiver, Recv };

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

    pub fn send(&self, msg: Vec<u8>) {
        self.channels.0.send(msg);
    }

    pub fn recv(&self) -> Recv<Vec<u8>> {
        self.channels.1.recv()
    }
}
