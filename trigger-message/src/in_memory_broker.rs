use std::sync::{
    atomic::{self, AtomicUsize},
    Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use spin_message_types::{InputMessage, OutputMessage};
use wildmatch::*;

use crate::broker::{create_channel, MessageBroker, QueueReceiver, QueueSender, Receiver, Sender};

#[derive(Clone, Debug)]
pub struct Subscription(WildMatch, Sender);

#[derive(Clone, Debug)]
pub struct QueueGroup(WildMatch, Vec<QueueSender>, Arc<AtomicUsize>);

#[derive(Clone, Debug, Default)]
pub struct InMemoryBroker {
    name: String,
    topic_subscriptions: Arc<DashMap<String, Subscription>>,
    queue_subscriptions: Arc<DashMap<String, QueueGroup>>,
}

impl InMemoryBroker {
    pub fn new(name: String) -> Self {
        Self {
            name,
            topic_subscriptions: Default::default(),
            queue_subscriptions: Default::default(),
        }
    }
}

#[async_trait]
impl MessageBroker for InMemoryBroker {
    fn name(&self) -> &str {
        &self.name
    }

    async fn publish(&self, message: OutputMessage) -> Result<()> {
        let subject = &message
            .subject
            .as_deref()
            .ok_or(anyhow::Error::msg("No Subject To Publish"))?;
        let message = InputMessage {
            message: message.message,
            subject: subject.to_string(),
            broker: self.name.clone(),
            response_subject: message.response_subject,
        };
        for r in self
            .topic_subscriptions
            .iter()
            .filter(|r| r.key() == subject || r.0.matches(subject))
        {
            let value = r.value();
            value.1.send(message.clone())?;
        }
        for r in self
            .queue_subscriptions
            .iter()
            .filter(|r| r.key() == subject || r.0.matches(subject))
        {
            let value = r.value();
            let mut index = value.2.fetch_add(1, atomic::Ordering::SeqCst);
            if index >= value.1.len() {
                value.2.store(1, atomic::Ordering::SeqCst);
                index = 0;
            }
            if let Some(sender) = value.1.get(index) {
                sender.send(message.clone())?;
            }
        }
        Ok(())
    }

    async fn subscribe_to_topic(&self, subject: &str) -> Result<Receiver> {
        if let Some(sender) = self.topic_subscriptions.get(subject) {
            Ok(sender.1.subscribe())
        } else {
            let sender = create_channel(10);
            let wildmatch = WildMatch::new(subject);
            self.topic_subscriptions
                .insert(subject.to_string(), Subscription(wildmatch, sender.clone()));
            Ok(sender.subscribe())
        }
    }

    async fn subscribe_to_queue(&self, topic: &str, group: &str) -> Result<QueueReceiver> {
        let subject = format!("{topic}::{group}");
        if let Some(mut group) = self.queue_subscriptions.get_mut(&subject) {
            let sender = create_channel(10);

            group.1.push(sender.clone());

            Ok(sender.subscribe())
        } else {
            let sender = create_channel(10);
            let wildmatch = WildMatch::new(topic);
            let group = QueueGroup(wildmatch, vec![sender.clone()], Arc::new(0.into()));
            self.queue_subscriptions.insert(subject.to_string(), group);
            Ok(sender.subscribe())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::broker::MessageBroker;
    use spin_message_types::OutputMessage;

    use super::InMemoryBroker;

    #[tokio::test]
    async fn a_published_message_gets_recieved_by_a_subscriber() {
        let message = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe_to_topic("message.test").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().unwrap();

        assert_eq!(result.subject, message.subject.unwrap());
        assert_eq!(result.message, message.message);
    }

    #[tokio::test]
    async fn a_published_message_doesnt_get_recieved_by_the_wrong_subscriber() {
        let message = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe_to_topic("message.wrong").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn multiple_published_messages_get_sent_through() {
        let message_1 = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };
        let message_2 = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test 2".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe_to_topic("message.test").await.unwrap();

        broker
            .publish_all(vec![message_1.clone(), message_2.clone()])
            .await
            .unwrap();

        let result = rx.try_recv().unwrap();
        assert_eq!(result.subject, "message.test");
        assert_eq!(result.message, message_1.message);

        let result = rx.try_recv().unwrap();
        assert_eq!(result.subject, "message.test");
        assert_eq!(result.message, message_2.message);
    }

    #[tokio::test]
    async fn a_wildcard_subscription_catches_matching_subjects() {
        let message = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe_to_topic("message.*").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().unwrap();

        assert_eq!(result.subject, message.subject.unwrap());
        assert_eq!(result.message, message.message);
    }

    #[tokio::test]
    async fn a_wildcard_subscription_doesnt_catch_non_matching_subjects() {
        let message = OutputMessage {
            subject: Some("test.message".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker.subscribe_to_topic("message.*").await.unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn a_published_message_on_a_queue_gets_recieved_by_a_subscriber() {
        let message = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx = broker
            .subscribe_to_queue("message.test", "group")
            .await
            .unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx.try_recv().expect("Should Successfully Recieve");

        assert_eq!(result.subject, message.subject.unwrap());
        assert_eq!(result.message, message.message);
    }

    #[tokio::test]
    async fn only_one_reciever_in_a_queue_group_recieves_a_message() {
        let message = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx1 = broker
            .subscribe_to_queue("message.test", "group")
            .await
            .unwrap();
        let mut rx2 = broker
            .subscribe_to_queue("message.test", "group")
            .await
            .unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx1.try_recv().expect("Should Successfully Recieve");
        let _ = rx2.try_recv().expect_err("Should Be Empty");

        assert_eq!(result.subject, message.subject.unwrap());
        assert_eq!(result.message, message.message);
    }

    #[tokio::test]
    async fn messages_alternate_between_members_of_queue_group() {
        let message = OutputMessage {
            subject: Some("message.test".to_string()),
            message: "test".as_bytes().to_owned(),
            broker: None,
            response_subject: None,
        };

        let broker = InMemoryBroker::default();

        let mut rx1 = broker
            .subscribe_to_queue("message.test", "group")
            .await
            .unwrap();
        let mut rx2 = broker
            .subscribe_to_queue("message.test", "group")
            .await
            .unwrap();

        broker.publish(message.clone()).await.unwrap();
        let result = rx1.try_recv().expect("Should Successfully Recieve");
        let _ = rx2.try_recv().expect_err("Should Be Empty");

        assert_eq!(&result.subject, message.subject.as_ref().unwrap());
        assert_eq!(result.message, message.message);

        broker.publish(message.clone()).await.unwrap();
        let result = rx2.try_recv().expect("Should Successfully Recieve");
        let _ = rx1.try_recv().expect_err("Should Be Empty");

        assert_eq!(&result.subject, message.subject.as_ref().unwrap());
        assert_eq!(result.message, message.message);

        broker.publish(message.clone()).await.unwrap();
        let result = rx1.try_recv().expect("Should Successfully Recieve");
        let _ = rx2.try_recv().expect_err("Should Be Empty");

        assert_eq!(&result.subject, message.subject.as_ref().unwrap());
        assert_eq!(result.message, message.message);
    }
}
