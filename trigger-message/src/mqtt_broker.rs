use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;

use rumqttc::{AsyncClient, ClientError, EventLoop, MqttOptions};
use serde::{Deserialize, Serialize};
use spin_message_types::OutputMessage;
use tokio::sync::mpsc;

use crate::{
    broker::{MessageBroker, QueueReceiver, Receiver, Sender},
    in_memory_broker::InMemoryBroker,
};

#[derive(Clone, Debug)]
pub struct Subscription(Sender);

#[derive(Clone, Debug)]
pub struct MqttBroker {
    name: String,
    local_broker: Arc<InMemoryBroker>,
    subscription_handler: mpsc::Sender<String>,
    queue_handler: mpsc::Sender<(String, String)>,
    publish_handler: mpsc::Sender<(String, OutputMessage)>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MqttCredentials {
    username: String,
    password: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MqttConnectionInfo {
    address: String,
    id: Option<String>,
    keep_alive: Option<f32>,
    credentials: Option<MqttCredentials>,
}

impl MqttConnectionInfo {
    pub async fn connect(&self) -> Result<(AsyncClient, EventLoop)> {
        let id = match &self.id {
            Some(id) => id.clone(),
            None => ulid::Ulid::new().to_string(),
        };
        let url = format!("{}?client_id={id}", self.address);
        let mut options = MqttOptions::parse_url(url)?;
        options.set_keep_alive(Duration::from_secs_f32(self.keep_alive.unwrap_or(5.)));

        if let Some(MqttCredentials { username, password }) = &self.credentials {
            options.set_credentials(username, password);
        }

        let (client, eventloop) = AsyncClient::new(options, 100);

        Ok((client, eventloop))
    }
}

impl MqttBroker {
    pub fn new(options: MqttConnectionInfo, name: String) -> Self {
        let local_broker = Arc::new(InMemoryBroker::default());
        let (subscription_handler, sub_rx) = mpsc::channel(100);
        let (publish_handler, pub_rx) = mpsc::channel(100);
        let (queue_handler, queue_rx) = mpsc::channel(100);
        let n = name.clone();
        let broker = local_broker.clone();
        tokio::spawn(async move {
            if let Err(e) =
                MqttBroker::setup_client(n, options, sub_rx, pub_rx, queue_rx, broker).await
            {
                eprintln!("Mqtt Error: {e}");
            }
        });

        Self {
            name,
            local_broker,
            subscription_handler,
            publish_handler,
            queue_handler,
        }
    }

    async fn setup_client(
        name: String,
        options: MqttConnectionInfo,
        mut sub_rx: mpsc::Receiver<String>,
        mut pub_rx: mpsc::Receiver<(String, OutputMessage)>,
        mut queue_rx: mpsc::Receiver<(String, String)>,
        local_broker: Arc<InMemoryBroker>,
    ) -> Result<()> {
        let (client, mut event_loop) = options.connect().await?;
        println!("Connected to MQTT for {name}");
        {
            let client = client.clone();
            tokio::spawn(async move {
                while let Some((subject, message)) = pub_rx.recv().await {
                    let Ok(body) = rmp_serde::to_vec(&message) else {
                        continue;
                    };
                    println!("Publishing on MQTT to {subject}");
                    let result: Result<(), ClientError> = client
                        .publish(subject.clone(), rumqttc::QoS::AtLeastOnce, false, body)
                        .await;
                    match result {
                        Ok(_) => println!("Published on MQTT to {subject}"),
                        Err(e) => eprintln!("Failed to publish on MQTT - {e:?}"),
                    };
                }
            });
        }

        {
            let client = client.clone();
            let _name = name.to_string();
            tokio::spawn(async move {
                while let Some((subject, group)) = queue_rx.recv().await {
                    println!("MQTT Queue Subscribe to {subject} {group}");
                    let _ = client
                        .subscribe(
                            format!("$share/{group}/{subject}"),
                            rumqttc::QoS::AtLeastOnce,
                        )
                        .await;
                }
            });
        }
        {
            let client = client.clone();
            let _name = name.to_string();
            tokio::spawn(async move {
                while let Some(subject) = sub_rx.recv().await {
                    println!("MQTT Subscribe to {subject}");
                    let _ = client
                        .subscribe(subject.to_string(), rumqttc::QoS::AtLeastOnce)
                        .await;
                }
            });
        }

        loop {
            match event_loop.poll().await {
                Ok(notification) => {
                    println!("MQTT Event {notification:?}");
                    if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(msg)) = notification {
                        let payload = msg.payload;
                        let Ok(message) = rmp_serde::from_slice(&payload) else {
                            continue;
                        };
                        let _ = local_broker.publish(message).await;
                    };
                }
                Err(e) => {
                    eprintln!("Disconnected from event loop, {e:?}");
                    break;
                }
            }
        }

        println!("exiting mqtt broker");

        Ok(())
    }
}

#[async_trait]
impl MessageBroker for MqttBroker {
    fn name(&self) -> &str {
        &self.name
    }

    async fn publish(&self, message: OutputMessage) -> Result<()> {
        let subject = &message
            .subject
            .as_deref()
            .ok_or(anyhow::Error::msg("No Subject To Publish"))?;
        self.publish_handler
            .send((subject.replace('.', "/").replace('*', "+"), message.clone()))
            .await?;
        Ok(())
    }

    async fn subscribe_to_topic(&self, subject: &str) -> Result<Receiver> {
        self.subscription_handler
            .send(subject.replace('.', "/").replace('*', "+"))
            .await?;
        self.local_broker.subscribe_to_topic(subject).await
    }

    async fn subscribe_to_queue(&self, topic: &str, group: &str) -> Result<QueueReceiver> {
        self.queue_handler
            .send((topic.replace('.', "/").replace('*', "+"), group.to_string()))
            .await?;
        self.local_broker.subscribe_to_queue(topic, group).await
    }
}
