use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Channel {
    pub name: String,
    pub broadcast: broadcast::Sender<Vec<u8>>,
    // pub connections: HashMap<String, Arc<Mutex<connection::Connection>>>,
}

impl Channel {
    pub fn new(name: &str) -> Self {
        let (tx, _rx) = broadcast::channel::<Vec<u8>>(64);
        Self {
            name: name.to_string(),
            broadcast: tx,
        }
    }
}
