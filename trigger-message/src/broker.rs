use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

use spin_message_types::SubjectMessage;

pub type Receiver = broadcast::Receiver<SubjectMessage>;
pub type Sender = broadcast::Sender<SubjectMessage>;

pub fn create_channel(capacity: usize) -> Sender {
    let (sender, _) = broadcast::channel(capacity);
    sender
}

#[async_trait]
pub trait MessageBroker: Send + Sync {
    async fn publish(&self, message: SubjectMessage) -> Result<()>;
    fn subscribe(&self, subject: &str) -> Result<Receiver>;

    async fn publish_all(&self, messages: Vec<SubjectMessage>) -> Result<()> {
        for msg in messages.into_iter() {
            self.publish(msg).await?;
        }
        Ok(())
    }
}
