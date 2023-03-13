use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::wit::{SubjectMessage, Message};

pub type Receiver = broadcast::Receiver<SubjectMessage>;
pub type Sender = broadcast::Sender<SubjectMessage>;

pub fn create_channel(capacity: usize) -> Sender {
    let (sender, _) = broadcast::channel(capacity);
    sender
}

#[async_trait]
pub trait MessageBroker {
    async fn publish(&self, message: SubjectMessage) -> Result<()>;
    fn subscribe(&mut self, subject: &str) -> Result<Receiver>;

    async fn publish_all(&self, subject: &str, messages: Vec<Message>) -> Result<()> {
        for msg in messages.iter() {
            self.publish(SubjectMessage {
                subject: subject.to_string(),
                message: msg.clone()
            }).await?;
        }
        Ok(())
    }
}
