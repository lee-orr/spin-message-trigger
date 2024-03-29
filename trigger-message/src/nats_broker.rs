use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use async_nats::{ConnectOptions, ServerAddr};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use spin_message_types::{InputMessage, OutputMessage};
use tokio::sync::{mpsc, oneshot};

use crate::broker::{create_channel, MessageBroker, QueueReceiver, Receiver, Sender};

#[derive(Clone, Debug)]
pub struct Subscription(Sender);

#[derive(Clone, Debug)]
pub struct NatsBroker {
    name: String,
    map: Arc<DashMap<String, Subscription>>,
    subscription_handler: mpsc::Sender<(String, Sender)>,
    queue_handler: mpsc::Sender<(String, String, Sender)>,
    publish_handler: mpsc::Sender<(String, OutputMessage)>,
    request_handler: mpsc::Sender<(String, OutputMessage, oneshot::Sender<InputMessage>)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NatsAuth {
    Token(String),
    User { user: String, password: String },
    NKey(String),
    Jwt { nkey_seed: String, jwt: String },
    CredentialsFile(String),
    CredentialsString(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientCertInfo {
    pub certificate: String,
    pub key: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NatsConnectionInfo {
    tls: Option<bool>,
    ping_interval: Option<u64>,
    addresses: Vec<String>,
    auth: Option<NatsAuth>,
    root_certificate: Option<String>,
    client_certificate: Option<ClientCertInfo>,
    client_name: Option<String>,
}

impl NatsConnectionInfo {
    pub async fn connect(&self) -> Result<async_nats::Client> {
        let mut options = if let Some(auth) = &self.auth {
            match auth {
                NatsAuth::Token(token) => ConnectOptions::with_token(token.clone()),
                NatsAuth::User { user, password } => {
                    ConnectOptions::with_user_and_password(user.clone(), password.clone())
                }
                NatsAuth::NKey(seed) => ConnectOptions::with_nkey(seed.clone()),
                NatsAuth::Jwt { nkey_seed, jwt } => {
                    let key_pair = std::sync::Arc::new(nkeys::KeyPair::from_seed(nkey_seed)?);

                    ConnectOptions::with_jwt(jwt.clone(), move |nonce| {
                        let key_pair = key_pair.clone();
                        async move { key_pair.sign(&nonce).map_err(async_nats::AuthError::new) }
                    })
                }
                NatsAuth::CredentialsFile(creds) => {
                    ConnectOptions::with_credentials_file(creds).await?
                }
                NatsAuth::CredentialsString(creds) => ConnectOptions::with_credentials(creds)?,
            }
        } else {
            ConnectOptions::new()
        };

        if let Some(tls) = self.tls {
            options = options.require_tls(tls);
        }

        if let Some(ping) = self.ping_interval {
            options = options.ping_interval(Duration::from_millis(ping));
        }

        if let Some(client_name) = &self.client_name {
            options = options.name(client_name.clone());
        }

        if let Some(path) = &self.root_certificate {
            let path = Path::new(path);
            let path = path.to_path_buf();
            options = options.add_root_certificates(path);
        }

        if let Some(ClientCertInfo { certificate, key }) = &self.client_certificate {
            let certificate = Path::new(certificate);
            let certificate = certificate.to_path_buf();
            let key = Path::new(key);
            let key = key.to_path_buf();
            options = options.add_client_certificate(certificate, key);
        }

        let addresses: Vec<ServerAddr> = self
            .addresses
            .iter()
            .filter_map(|v| {
                if let Ok(v) = v.parse::<ServerAddr>() {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();
        Ok(options.connect(addresses).await?)
    }
}

impl NatsBroker {
    pub fn new(options: NatsConnectionInfo, name: String) -> Self {
        let (subscription_handler, sub_rx) = mpsc::channel(100);
        let (publish_handler, pub_rx) = mpsc::channel(100);
        let (request_handler, req_rx) = mpsc::channel(100);
        let (queue_handler, queue_rx) = mpsc::channel(100);
        let n = name.clone();
        tokio::spawn(async move {
            if let Err(e) =
                NatsBroker::setup_client(n, options, sub_rx, pub_rx, req_rx, queue_rx).await
            {
                eprintln!("Nats Error: {e}");
            }
        });

        Self {
            name,
            map: Default::default(),
            subscription_handler,
            publish_handler,
            request_handler,
            queue_handler,
        }
    }

    async fn setup_client(
        name: String,
        options: NatsConnectionInfo,
        mut sub_rx: mpsc::Receiver<(String, Sender)>,
        mut pub_rx: mpsc::Receiver<(String, OutputMessage)>,
        mut req_rx: mpsc::Receiver<(String, OutputMessage, oneshot::Sender<InputMessage>)>,
        mut queue_rx: mpsc::Receiver<(String, String, Sender)>,
    ) -> Result<()> {
        let client = options.connect().await?;
        println!("Connected to NATS for {name}");
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
        {
            let client = client.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                while let Some((subject, message, response)) = req_rx.recv().await {
                    let body = message.message;
                    println!("Requesting on NATS to {subject}");
                    let request = async_nats::Request::new().payload(body.into());
                    let result = client.send_request(subject.clone(), request).await;
                    match result {
                        Ok(msg) => {
                            let subject = msg.subject.clone();
                            println!("Received NATS Response on {subject}");
                            let body = msg.payload.to_vec();

                            let _ = response.send(InputMessage {
                                message: body,
                                subject: subject.to_string(),
                                broker: name.clone(),
                                response_subject: msg.reply.map(|v| v.to_string()),
                            });
                        }
                        Err(e) => {
                            eprintln!("Request/Response Failed - {e:?}");
                        }
                    }
                }
            });
        }
        {
            let client = client.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                while let Some((subject, group, sender)) = queue_rx.recv().await {
                    let client = client.clone();
                    let name = name.to_string();
                    tokio::spawn(async move {
                        if let Ok(mut pubsub) =
                            client.queue_subscribe(subject.clone(), group.clone()).await
                        {
                            println!("Queue subscribed to async_nats: {subject} - {group}");
                            while let Some(msg) = pubsub.next().await {
                                let subject = msg.subject.clone();
                                println!("Received Queued NATS Message on {subject} - {group}");
                                let body = msg.payload.to_vec();
                                let _ = sender.send(InputMessage {
                                    message: body,
                                    subject: subject.to_string(),
                                    broker: name.clone(),
                                    response_subject: msg.reply.map(|v| v.to_string()),
                                });
                            }
                        }
                    });
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
                        let subject = msg.subject.clone();
                        println!("Received NATS Message on {subject}");
                        let body = msg.payload.to_vec();
                        let _ = sender.send(InputMessage {
                            message: body,
                            subject: subject.to_string(),
                            broker: name.clone(),
                            response_subject: msg.reply.map(|v| v.to_string()),
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

    async fn request(&self, request: OutputMessage) -> Result<InputMessage> {
        let Some(subject) = request.subject.clone() else {
            bail!("No subject set");
        };

        let (responder, reciever) = oneshot::channel();
        self.request_handler
            .send((subject.to_string(), request, responder))
            .await?;

        let Ok(result) = reciever.await else {
            bail!("couldn't get result");
        };

        Ok(result)
    }
}
