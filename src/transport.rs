use async_trait::async_trait;

#[async_trait]
pub trait Transport {
    async fn process(self) -> Result<(), anyhow::Error>;
    fn close(&self);
}
