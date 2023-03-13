use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;
use dashmap::DashMap;

use crate::{broker::{MessageBroker, Sender, Receiver, create_channel}, wit::SubjectMessage};

#[derive(Clone, Debug, Default)]
pub struct InMemoryBroker {
    map: Arc<DashMap<String, Sender>>
}

#[async_trait]
impl MessageBroker for InMemoryBroker {
    async fn publish(&self, message: SubjectMessage) -> Result<()> {
        let subject = &message.subject;
        self.map.iter().filter(|r| r.key() == subject);
        todo!()
    }

    fn subscribe(
        &mut self,
        subject: &str,
    ) -> Result<Receiver> {
        if let Some(sender) = self.map.get(subject) {
            Ok(sender.subscribe())
        } else {
            let sender = create_channel(10);
            self.map.insert(subject.to_string(), sender.clone());
            Ok(sender.subscribe())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{wit::{SubjectMessage, Message}, broker::MessageBroker};

    use super::InMemoryBroker;

    #[tokio::test]
    async fn a_published_message_gets_recieved_by_a_subscriber() {
        let message = SubjectMessage {
            subject: format!("message.test"),
            message: Message { body: Some("test".as_bytes().to_owned()), metadata: vec![] },
        };

        let mut broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.test").unwrap();
    
        let publish = broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().unwrap();

        assert_eq!(result.subject, message.subject);
        assert_eq!(result.message.body, message.message.body);
    }
}