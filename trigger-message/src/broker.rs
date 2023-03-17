use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

use spin_message_types::{InputMessage, OutputMessage};

pub type Receiver = broadcast::Receiver<InputMessage>;
pub type Sender = broadcast::Sender<InputMessage>;

pub fn create_channel(capacity: usize) -> Sender {
    let (sender, _) = broadcast::channel(capacity);
    sender
}

#[async_trait]
pub trait MessageBroker: Send + Sync {
    fn name(&self) -> &str;
    async fn publish(&self, message: OutputMessage) -> Result<()>;
    async fn subscribe(&self, subject: &str) -> Result<Receiver>;

    async fn publish_all(&self, messages: Vec<OutputMessage>) -> Result<()> {
        for msg in messages.into_iter() {
            self.publish(msg).await?;
        }
        Ok(())
    }
}
