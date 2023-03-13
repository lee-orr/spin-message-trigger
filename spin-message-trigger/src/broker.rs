use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{broadcast};

use crate::wit::SubjectMessage;

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
}