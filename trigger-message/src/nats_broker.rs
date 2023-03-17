use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::StreamExt;
use spin_message_types::{InputMessage, OutputMessage};
use tokio::sync::mpsc;

use crate::broker::{create_channel, MessageBroker, Receiver, Sender};

#[derive(Clone, Debug)]
pub struct Subscription(Sender);

#[derive(Clone, Debug)]
pub struct NatsBroker {
    name: String,
    map: Arc<DashMap<String, Subscription>>,
    subscription_handler: mpsc::Sender<(String, Sender)>,
    publish_handler: mpsc::Sender<(String, OutputMessage)>,
}

impl NatsBroker {
    pub fn new(address: String, name: String) -> Self {
        let (subscription_handler, sub_rx) = mpsc::channel(100);
        let (publish_handler, pub_rx) = mpsc::channel(100);
        let n = name.clone();
        tokio::spawn(async move {
            if let Err(e) = NatsBroker::setup_client(n, address, sub_rx, pub_rx).await {
                eprintln!("Nats Error: {e}");
            }
        });

        Self {
            name,
            map: Default::default(),
            subscription_handler,
            publish_handler,
        }
    }

    async fn setup_client(
        name: String,
        address: String,
        mut sub_rx: mpsc::Receiver<(String, Sender)>,
        mut pub_rx: mpsc::Receiver<(String, OutputMessage)>,
    ) -> Result<()> {
        let client = async_nats::connect(&address).await?;
        println!("Connected to NATS at {address}");
        {
            let client = client.clone();
            tokio::spawn(async move {
                while let Some((subject, message)) = pub_rx.recv().await {
                    let body = message.message;
                    println!("Publishing on NATS to {subject}");
                    let result = client.publish(subject.clone(), body.into());
                    match result.await {
                        Ok(_) => println!("Published on NATS to {subject}"),
                        Err(e) => eprintln!("Failed to publish on NATS - {e:?}"),
                    }
                }
            });
        }

        while let Some((subject, sender)) = sub_rx.recv().await {
            let client = client.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                if let Ok(mut pubsub) = client.subscribe(subject.clone()).await {
                    println!("Subscribed to async_nats: {subject}");
                    while let Some(msg) = pubsub.next().await {
                        println!("Received NATS Message on {subject}");
                        let body = msg.payload.to_vec();
                        let _ = sender.send(InputMessage {
                            message: body,
                            subject: subject.clone(),
                            broker: name.clone(),
                        });
                    }
                }
            });
        }

        Ok(())
    }
}

#[async_trait]
impl MessageBroker for NatsBroker {
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

    async fn subscribe(&self, subject: &str) -> Result<Receiver> {
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
}
