#[derive(Debug, Clone)]
pub enum Message {
    CreateTransport(CreateTransportRequest),
    DestroyTransport(DestroyTransportRequest),
}

#[derive(Debug, Clone)]
pub struct CreateTransportRequest {}

#[derive(Debug, Clone)]
pub struct DestroyTransportRequest {}
