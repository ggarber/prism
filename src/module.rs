use tokio::sync::broadcast;

// pub trait Module {

// }

pub type MessageData = String;

#[derive(Clone, Debug)]
pub struct Message {
    pub data: MessageData,
    pub reply: broadcast::Sender<MessageData>,
}

#[derive(Debug)]
pub struct Module {
    pub commands: broadcast::Sender<Message>,
    pub events: broadcast::Sender<MessageData>,
}

impl Module {
    pub fn new() -> Self {
        let (commands, _) = broadcast::channel::<Message>(64);
        let (events, _) = broadcast::channel::<MessageData>(64);
        Self { commands, events }
    }
}
