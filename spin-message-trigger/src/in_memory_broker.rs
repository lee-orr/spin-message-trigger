use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;

use crate::{
    broker::{create_channel, MessageBroker, Receiver, Sender},
    wit::SubjectMessage,
};

#[derive(Clone, Debug, Default)]
pub struct InMemoryBroker {
    map: Arc<DashMap<String, Sender>>,
}

#[async_trait]
impl MessageBroker for InMemoryBroker {
    async fn publish(&self, message: SubjectMessage) -> Result<()> {
        let subject = &message.subject;
        for r in self.map.iter().filter(|r| r.key() == subject) {
            let value = r.value();
            value.send(message.clone())?;
        }
        Ok(())
    }

    fn subscribe(&mut self, subject: &str) -> Result<Receiver> {
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
    use crate::{
        broker::MessageBroker,
        wit::{Message, SubjectMessage},
    };

    use super::InMemoryBroker;

    #[tokio::test]
    async fn a_published_message_gets_recieved_by_a_subscriber() {
        let message = SubjectMessage {
            subject: "message.test".to_string(),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
        };

        let mut broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.test").unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().unwrap();

        assert_eq!(result.subject, message.subject);
        assert_eq!(result.message.body, message.message.body);
    }

    #[tokio::test]
    async fn a_published_message_doesnt_get_recieved_by_the_wrong_subscriber() {
        let message = SubjectMessage {
            subject: "message.test".to_string(),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
        };

        let mut broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.wrong").unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn multiple_published_messages_get_sent_through() {
        let message_1 = Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            };
        let message_2 = Message {
                body: Some("test 2".as_bytes().to_owned()),
                metadata: vec![],
            };

        let mut broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.test").unwrap();

        broker.publish_all("message.test", vec![message_1.clone(), message_2.clone()]).await.unwrap();

        let result = rx.try_recv().unwrap();
        assert_eq!(result.subject, "message.test");
        assert_eq!(result.message.body, message_1.body);

        let result = rx.try_recv().unwrap();
        assert_eq!(result.subject, "message.test");
        assert_eq!(result.message.body, message_2.body);
    }
}