use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use spin_message_types::SubjectMessage;
use wildmatch::*;

use crate::broker::{create_channel, MessageBroker, Receiver, Sender};

#[derive(Clone, Debug)]
pub struct Subscription(WildMatch, Sender);

#[derive(Clone, Debug, Default)]
pub struct InMemoryBroker {
    map: Arc<DashMap<String, Subscription>>,
}

#[async_trait]
impl MessageBroker for InMemoryBroker {
    async fn publish(&self, message: SubjectMessage) -> Result<()> {
        let subject = &message
            .subject
            .as_deref()
            .ok_or(anyhow::Error::msg("No Subject To Publish"))?;
        for r in self
            .map
            .iter()
            .filter(|r| r.key() == subject || r.0.matches(subject))
        {
            let value = r.value();
            value.1.send(message.clone())?;
        }
        Ok(())
    }

    async fn subscribe(&self, subject: &str) -> Result<Receiver> {
        if let Some(sender) = self.map.get(subject) {
            Ok(sender.1.subscribe())
        } else {
            let sender = create_channel(10);
            let wildmatch = WildMatch::new(subject);
            self.map
                .insert(subject.to_string(), Subscription(wildmatch, sender.clone()));
            Ok(sender.subscribe())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::broker::MessageBroker;
    use spin_message_types::{Message, SubjectMessage};

    use super::InMemoryBroker;

    #[tokio::test]
    async fn a_published_message_gets_recieved_by_a_subscriber() {
        let message = SubjectMessage {
            subject: Some("message.test".to_string()),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
            broker: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.test").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().unwrap();

        assert_eq!(result.subject, message.subject);
        assert_eq!(result.message.body, message.message.body);
    }

    #[tokio::test]
    async fn a_published_message_doesnt_get_recieved_by_the_wrong_subscriber() {
        let message = SubjectMessage {
            subject: Some("message.test".to_string()),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
            broker: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.wrong").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn multiple_published_messages_get_sent_through() {
        let message_1 = SubjectMessage {
            subject: Some("message.test".to_string()),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
            broker: None,
        };
        let message_2 = SubjectMessage {
            subject: Some("message.test".to_string()),
            message: Message {
                body: Some("test 2".as_bytes().to_owned()),
                metadata: vec![],
            },
            broker: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.test").await.unwrap();

        broker
            .publish_all(vec![message_1.clone(), message_2.clone()])
            .await
            .unwrap();

        let result = rx.try_recv().unwrap();
        assert_eq!(result.subject.unwrap(), "message.test");
        assert_eq!(result.message.body, message_1.message.body);

        let result = rx.try_recv().unwrap();
        assert_eq!(result.subject.unwrap(), "message.test");
        assert_eq!(result.message.body, message_2.message.body);
    }

    #[tokio::test]
    async fn a_wildcard_subscription_catches_matching_subjects() {
        let message = SubjectMessage {
            subject: Some("message.test".to_string()),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
            broker: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.*").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().unwrap();

        assert_eq!(result.subject, message.subject);
        assert_eq!(result.message.body, message.message.body);
    }

    #[tokio::test]
    async fn a_wildcard_subscription_doesnt_catch_non_matching_subjects() {
        let message = SubjectMessage {
            subject: Some("test.message".to_string()),
            message: Message {
                body: Some("test".as_bytes().to_owned()),
                metadata: vec![],
            },
            broker: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe("message.*").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv();
        assert!(result.is_err());
    }
}
