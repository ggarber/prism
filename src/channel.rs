use crossbeam_channel::{ Sender, Receiver };

#[derive(Debug)]
pub struct Channel {
    channels: (Sender<Vec<u8>>, Receiver<Vec<u8>>),
}

impl Channel {
    pub fn new(name: &str) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self {
            channels: (tx, rx),
        }
    }

    pub fn send(&self, msg: Vec<u8>) {
        self.channels.0.send(msg);
    }

    pub fn recv(&self) -> Vec<u8> {
        self.channels.1.recv().unwrap()
    }
}
