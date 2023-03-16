mod broker;
mod gateway;
mod in_memory_broker;
mod redis_broker;

use anyhow::{bail, Error};
use broker::MessageBroker;
use clap::Parser;
use gateway::spawn_gateway;
use serde::{Deserialize, Serialize};
use spin_trigger::{cli::TriggerExecutorCommand, TriggerAppEngine, TriggerExecutor};
use std::{collections::HashMap, sync::Arc};

use in_memory_broker::InMemoryBroker;
use spin_message_types::export::messages::{
    InternalMessageParam, InternalMetadataParam, InternalSubjectMessageParam, Outcome,
};

use spin_message_types::SubjectMessage;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Setting up Message Trigger");
    let t = Command::parse();
    t.run().await
}

type Command = TriggerExecutorCommand<MessageTrigger>;
type RuntimeData = spin_message_types::export::messages::ExportedData;

pub enum Broker {
    InMemoryBroker(InMemoryBroker),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BrokerConfig {
    pub broker_type: BrokerTypeConfig,
    pub gateway: GatewayConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum BrokerTypeConfig {
    #[default]
    InMemoryBroker,
    Redis(String)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum WebsocketConfig {
    #[default]
    BinaryBody,
    TextBody,
    Messagepack,
    Json,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum GatewayConfig {
    #[default]
    None,
    Http {
        port: u16,
        websockets: Option<WebsocketConfig>,
    },
}

struct MessageTrigger {
    engine: TriggerAppEngine<Self>,
    brokers: HashMap<String, Arc<dyn MessageBroker>>,
    components: Vec<MessageTriggerConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MessageMetadata {
    r#type: String,
    brokers: HashMap<String, BrokerConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MessageResultType {
    default_broker: String,
    default_subject: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageTriggerConfig {
    component: String,
    broker: String,
    subscription: String,
    result: Option<MessageResultType>,
}

#[async_trait::async_trait]
impl TriggerExecutor for MessageTrigger {
    const TRIGGER_TYPE: &'static str = "message";

    type RuntimeData = RuntimeData;

    type TriggerConfig = MessageTriggerConfig;

    type RunConfig = spin_trigger::cli::NoArgs;

    fn new(engine: TriggerAppEngine<Self>) -> anyhow::Result<Self> {
        println!("Getting metadata - let's see what it is...");
        let metadata = engine
            .app()
            .require_metadata::<MessageMetadata>("trigger")?;
        println!("Getting Trigger Configs");
        let components = engine
            .trigger_configs()
            .map(|(_, config)| config.clone())
            .collect();
        println!("Setting Up Brokers");
        let brokers = metadata
            .brokers
            .iter()
            .map(
                |(
                    key,
                    BrokerConfig {
                        broker_type,
                        gateway,
                    },
                )| {
                    println!(
                        "Setting up {key} - with broker {broker_type:?} and gateway {gateway:?}"
                    );
                    let key = key.clone();
                    let broker: Arc<dyn MessageBroker> = match broker_type {
                        BrokerTypeConfig::InMemoryBroker => {
                            Arc::<in_memory_broker::InMemoryBroker>::default()
                        }
                        BrokerTypeConfig::Redis(address) => {
                            Arc::new(redis_broker::RedisBroker::new(address.clone()))
                        }
                    };
                    if let GatewayConfig::Http { port, websockets } = gateway {
                        tokio::spawn(spawn_gateway(*port, websockets.clone(), broker.clone()));
                    }
                    println!("Broker and gateway for key {key} complete");
                    (key, broker)
                },
            )
            .collect();
        Ok(Self {
            engine,
            components,
            brokers,
        })
    }

    async fn run(self, _config: Self::RunConfig) -> anyhow::Result<()> {
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            std::process::exit(0);
        });

        tokio_scoped::scope(|scope| {
            // For each component, run its own timer loop
            for config in &self.components {
                scope.spawn(async {
                    let config = config.clone();
                    let rx = if let Some(broker) = self.brokers.get(&config.broker) {
                        broker.subscribe(&config.subscription).await.ok()
                    } else {
                        None
                    };

                    if let Some(mut rx) = rx {
                        while let Ok(message) = rx.recv().await {
                            println!("Got message {message:?}");
                            if let Err(e) = self.handle_message(&config, message).await {
                                eprintln!("Error handling message: {e:?}");
                            }
                        }
                    }
                });
            }
        });
        Ok(())
    }
}

impl MessageTrigger {
    async fn send_with_broker(
        &self,
        broker: &str,
        subject: &str,
        mut msg: SubjectMessage,
    ) -> anyhow::Result<()> {
        let broker = msg.broker.as_deref().unwrap_or(broker);
        if let Some(broker) = self.brokers.get(broker) {
            msg.subject = Some(msg.subject.unwrap_or(subject.to_string()));
            broker.publish(msg).await?;
            Ok(())
        } else {
            bail!("No such broker");
        }
    }
    async fn send_all_with_broker(
        &self,
        broker: &str,
        subject: &str,
        msgs: Vec<SubjectMessage>,
    ) -> anyhow::Result<()> {
        for msg in msgs.into_iter() {
            if let Err(e) = self.send_with_broker(broker, subject, msg).await {
                eprintln!("Error sending message: {e:?}");
            }
        }
        Ok(())
    }

    async fn handle_message(
        &self,
        config: &MessageTriggerConfig,
        message: SubjectMessage,
    ) -> anyhow::Result<()> {
        let (instance, mut store) = self.engine.prepare_instance(&config.component).await?;
        println!("Setup instance");
        let engine =
            spin_message_types::export::messages::Exported::new(&mut store, &instance, |data| {
                data.as_mut()
            })?;
        println!("engine ready");

        let metadata = message
            .message
            .metadata
            .iter()
            .map(|v| InternalMetadataParam {
                name: &v.name,
                value: v.value.as_slice(),
            })
            .collect::<Vec<_>>();

        let original_subject = &message.subject;

        let message = InternalSubjectMessageParam {
            subject: message.subject.as_deref(),
            message: InternalMessageParam {
                body: message.message.body.as_deref(),
                metadata: metadata.as_slice(),
            },
            broker: Some(&config.broker),
        };

        println!("ready for wasm");

        let result = engine.handle_message(&mut store, message).await?;

        println!("Got result {result:?}");

        match (result, &config.result) {
            (
                Outcome::Publish(msgs),
                Some(MessageResultType {
                    default_broker,
                    default_subject,
                }),
            ) => {
                self.send_all_with_broker(
                    default_broker,
                    default_subject,
                    msgs.into_iter().map(|v| v.into()).collect(),
                )
                .await?
            }
            (Outcome::Publish(msgs), None) => {
                if let Some(default_subject) = original_subject {
                    self.send_all_with_broker(
                        &config.broker,
                        default_subject,
                        msgs.into_iter().map(|v| v.into()).collect(),
                    )
                    .await?
                }
            }
            _ => {}
        }
        Ok(())
    }
}
