use crate::broker::MessageBroker;
use crate::configs::*;
use anyhow::bail;

use crate::gateway::spawn_gateway;

use serde::{Deserialize, Serialize};
use spin_app::MetadataKey;
use spin_message_types::export::{InternalMessage, Outcome};
use spin_trigger::EitherInstance;
use spin_trigger::{cli::TriggerExecutorCommand, TriggerAppEngine, TriggerExecutor};
use std::{collections::HashMap, sync::Arc};

use spin_message_types::{InputMessage, OutputMessage};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageMetadata {
    r#type: String,
    brokers: HashMap<String, BrokerConfig>,
}

pub struct MessageTrigger {
    engine: TriggerAppEngine<Self>,
    brokers: HashMap<String, Arc<dyn MessageBroker>>,
    components: Vec<MessageTriggerConfig>,
}

pub type Command = TriggerExecutorCommand<MessageTrigger>;
pub type RuntimeData = ();

const TRIGGER_METADATA_KEY: MetadataKey<MessageMetadata> = MetadataKey::new("trigger");

#[async_trait::async_trait]
impl TriggerExecutor for MessageTrigger {
    const TRIGGER_TYPE: &'static str = "message";

    type RuntimeData = RuntimeData;

    type TriggerConfig = MessageTriggerConfig;

    type RunConfig = spin_trigger::cli::NoArgs;

    async fn new(engine: TriggerAppEngine<Self>) -> anyhow::Result<Self> {
        println!("Getting metadata - let's see what it is...");
        let metadata = engine.app().require_metadata(TRIGGER_METADATA_KEY)?;
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
                            Arc::new(crate::in_memory_broker::InMemoryBroker::new(key.clone()))
                        }
                        BrokerTypeConfig::Redis(address) => Arc::new(
                            crate::redis_broker::RedisBroker::new(address.clone(), key.clone()),
                        ),
                        BrokerTypeConfig::Nats(options) => Arc::new(
                            crate::nats_broker::NatsBroker::new(options.clone(), key.clone()),
                        ),
                    };
                    if let GatewayConfig::Http {
                        port,
                        websockets,
                        request_response,
                        timeout,
                    } = gateway
                    {
                        tokio::spawn(spawn_gateway(
                            *port,
                            websockets.clone(),
                            broker.clone(),
                            request_response.clone(),
                            *timeout,
                        ));
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
        println!("Running message trigger");
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
        mut msg: OutputMessage,
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
        msgs: Vec<OutputMessage>,
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
        message: InputMessage,
    ) -> anyhow::Result<()> {
        let (instance, mut store) = self.engine.prepare_instance(&config.component).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };
        println!("Setup instance");
        let instance = spin_message_types::export::SpinMessageTrigger::new(&mut store, &instance)?;
        println!("engine ready");

        let original_subject = &message.subject;

        let message = InternalMessage {
            subject: &message.subject,
            message: &message.message,
            broker: &config.broker,
            response_subject: message.response_subject.as_deref(),
        };

        println!("ready for wasm");

        let result = instance
            .guest()
            .call_handle_message(&mut store, message)
            .await?;

        println!("Got result {result:?}");

        let default_result_target = match &config.subscription {
            SubscriptionType::Topic { topic: _, result } => result.as_ref(),
            SubscriptionType::Queue {
                topic: _,
                group: _,
                result,
            } => result.as_ref(),
            _ => None,
        };

        match (result, default_result_target) {
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
                self.send_all_with_broker(
                    &config.broker,
                    original_subject,
                    msgs.into_iter().map(|v| v.into()).collect(),
                )
                .await?
            }
            _ => {}
        }
        Ok(())
    }
}
