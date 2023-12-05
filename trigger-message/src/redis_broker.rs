use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::StreamExt;
use rsmq_async::{Rsmq, RsmqConnection};
use spin_message_types::{InputMessage, OutputMessage};
use tokio::sync::mpsc;

use crate::broker::{
    create_channel, default_message_response_subject, MessageBroker, QueueReceiver, Receiver,
    Sender,
};
use redis::*;

#[derive(Clone, Debug)]
pub struct Subscription(Sender);

#[derive(Clone, Debug)]
pub struct RedisBroker {
    name: String,
    map: Arc<DashMap<String, Subscription>>,
    subscription_handler: mpsc::Sender<(String, Sender)>,
    publish_handler: mpsc::Sender<(String, OutputMessage)>,
    queue_handler: mpsc::Sender<(String, String, Sender)>,
}

impl RedisBroker {
    pub fn new(address: String, name: String) -> Self {
        let (subscription_handler, sub_rx) = mpsc::channel(100);
        let (publish_handler, pub_rx) = mpsc::channel(100);
        let (queue_handler, queue_rx) = mpsc::channel(100);
        let n = name.clone();
        tokio::spawn(async move {
            if let Err(e) = RedisBroker::setup_client(n, address, sub_rx, pub_rx, queue_rx).await {
                eprintln!("Redis Error: {e}");
            }
        });

        Self {
            name,
            map: Default::default(),
            subscription_handler,
            publish_handler,
            queue_handler,
        }
    }

    async fn setup_client(
        name: String,
        address: String,
        mut sub_rx: mpsc::Receiver<(String, Sender)>,
        mut pub_rx: mpsc::Receiver<(String, OutputMessage)>,
        mut queue_rx: mpsc::Receiver<(String, String, Sender)>,
    ) -> Result<()> {
        let client = redis::Client::open(address)?;
        {
            let cloned = client.clone();
            tokio::spawn(async move {
                while let Some((subject, message)) = pub_rx.recv().await {
                    if let Ok(mut connection) = cloned.get_tokio_connection().await {
                        println!("Publish redis connection ready");
                        let body = message.message;
                        println!("Publishing to {subject}");
                        let result: RedisFuture<Value> = connection.publish(subject.clone(), &body);
                        match result.await {
                            Ok(_) => println!("Published to {subject}"),
                            Err(e) => eprintln!("Failed to publish - {e:?}"),
                        }
                        let mut rsmq = Rsmq::new_with_connection(connection, true, Some(&subject));
                        let Ok(queues) = rsmq.list_queues().await else {
                            continue;
                        };
                        for qname in queues.iter() {
                            let result = rsmq.send_message(qname, body.clone(), None).await;
                            match result {
                                Ok(_) => println!("Published to queue {subject} - {qname}"),
                                Err(e) => eprintln!("Failed to publish queue - {e:?}"),
                            }
                        }
                    }
                }
            });
        }

        {
            let client = client.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                while let Some((subject, sender)) = sub_rx.recv().await {
                    let cloned = client.clone();
                    let name = name.to_string();
                    tokio::spawn(async move {
                        if let Ok(connection) = cloned.get_tokio_connection().await {
                            println!("Subscribed to redis: {subject}");
                            let mut pubsub = connection.into_pubsub();
                            if let Ok(()) = pubsub.psubscribe(subject.clone()).await {
                                let mut msgs = pubsub.on_message();
                                while let Some(msg) = msgs.next().await {
                                    let body = msg.get_payload_bytes().to_owned();
                                    let _ = sender.send(InputMessage {
                                        message: body,
                                        subject: subject.clone(),
                                        broker: name.clone(),
                                        response_subject: default_message_response_subject(
                                            &subject,
                                        ),
                                    });
                                }
                            }
                        }
                    });
                }
            });
        }

        while let Some((subject, group, sender)) = queue_rx.recv().await {
            let cloned = client.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                let Ok(connection) = cloned.get_tokio_connection().await else {
                    eprintln!("Couldn't get redis connectio - for queue n");
                    return;
                };
                let mut rsmq = Rsmq::new_with_connection(connection, true, Some(&subject));
                match rsmq.create_queue(&group, None, None, None).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error creating queue - {subject} - {group}: {e:?}");
                    }
                };

                let subscription = format!("{subject}:rt:{group}");

                let Ok(connection) = cloned.get_tokio_connection().await else {
                    eprintln!("Couldn't get redis connectio - for queue n");
                    return;
                };
                let mut pubsub = connection.into_pubsub();
                if let Ok(()) = pubsub.psubscribe(subscription.clone()).await {
                    let mut msgs = pubsub.on_message();
                    println!("Subscribed to redis queue: {subject} - {group}");
                    while (msgs.next().await).is_some() {
                        if let Ok(Some(body)) = rsmq.pop_message::<Vec<u8>>(&group).await {
                            let _ = sender.send(InputMessage {
                                message: body.message,
                                subject: subject.clone(),
                                broker: name.clone(),
                                response_subject: default_message_response_subject(&subject),
                            });
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

#[async_trait]
impl MessageBroker for RedisBroker {
    fn name(&self) -> &str {
        &self.name
    }

    async fn publish(&self, message: OutputMessage) -> Result<()> {
        let subject = &message
            .subject
            .as_deref()
            .ok_or(anyhow::Error::msg("No Subject To Publish"))?;
        self.publish_handler
            .send((subject.to_string(), message))
            .await?;
        Ok(())
    }

    async fn subscribe_to_topic(&self, subject: &str) -> Result<Receiver> {
        if let Some(sender) = self.map.get(subject) {
            Ok(sender.0.subscribe())
        } else {
            let sender = create_channel(100);
            self.map
                .insert(subject.to_string(), Subscription(sender.clone()));
            self.subscription_handler
                .send((subject.to_string(), sender.clone()))
                .await?;
            Ok(sender.subscribe())
        }
    }

    async fn subscribe_to_queue(&self, topic: &str, group: &str) -> Result<QueueReceiver> {
        let sender = create_channel(100);
        self.queue_handler
            .send((topic.to_string(), group.to_string(), sender.clone()))
            .await?;
        Ok(sender.subscribe())
    }
}
