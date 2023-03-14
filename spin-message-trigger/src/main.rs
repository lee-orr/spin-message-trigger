mod broker;
mod in_memory_broker;
mod wit;

use anyhow::{Error, bail};
use broker::MessageBroker;
use clap::Parser;
use serde::{Deserialize, Serialize};
use spin_trigger::{cli::TriggerExecutorCommand, TriggerAppEngine, TriggerExecutor};
use std::collections::HashMap;

use in_memory_broker::InMemoryBroker;
use wit::{
    messages::{MessageParam, MetadataParam, SubjectMessageParam, Outcome},
    SubjectMessage, Message,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let t = Command::parse();
    t.run().await
}

type Command = TriggerExecutorCommand<MessageTrigger>;
type RuntimeData = wit::messages::MessagesData;

pub enum Broker {
    InMemoryBroker(InMemoryBroker),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum BrokerConfig {
    #[default]
    InMemoryBroker,
}

struct MessageTrigger {
    engine: TriggerAppEngine<Self>,
    brokers: HashMap<String, Box<dyn MessageBroker>>,
    components: Vec<MessageTriggerConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MessageMetadata {
    broker_configs: HashMap<String, BrokerConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum MessageResultType {
    #[default]
    None,
    Response {
        broker: String,
        subject: String,
    },
    Publish {
        broker: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageTriggerConfig {
    component: String,
    broker: String,
    subscription: String,
    result: MessageResultType,
}

#[async_trait::async_trait]
impl TriggerExecutor for MessageTrigger {
    const TRIGGER_TYPE: &'static str = "message";

    type RuntimeData = RuntimeData;

    type TriggerConfig = MessageTriggerConfig;

    type RunConfig = spin_trigger::cli::NoArgs;

    fn new(engine: TriggerAppEngine<Self>) -> anyhow::Result<Self> {
        let metadata = engine
            .app()
            .require_metadata::<MessageMetadata>("trigger")?;
        let components = engine
            .trigger_configs()
            .map(|(_, config)| config.clone())
            .collect();
        let brokers = metadata
            .broker_configs
            .iter()
            .map(|(key, value)| {
                let key = key.clone();
                let broker: Box<dyn MessageBroker> = match value {
                    BrokerConfig::InMemoryBroker => {
                        Box::<in_memory_broker::InMemoryBroker>::default()
                    }
                };
                (key, broker)
            })
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
                        broker.subscribe(&config.subscription).ok()
                    } else {
                        None
                    };

                    if let Some(mut rx) = rx {
                        while let Ok(message) = rx.recv().await {
                            let _ = self.handle_message(&config, message).await;
                        }
                    }
                });
            }
        });
        Ok(())
    }
}

impl MessageTrigger {
    async fn send_with_broker(&self, broker: &String, msg: SubjectMessage) -> anyhow::Result<()>{
        if let Some(broker) = self.brokers.get(broker) {
            broker.publish(msg).await?;
            Ok(())
        } else {
            bail!("No such broker");
        }
    }
    async fn send_all_with_broker(&self, broker: &String, msg: Vec<SubjectMessage>) -> anyhow::Result<()>{
        if let Some(broker) = self.brokers.get(broker) {
            broker.publish_all(msg).await?;
            Ok(())
        } else {
            bail!("No such broker");
        }
    }

    async fn handle_message(
        &self,
        config: &MessageTriggerConfig,
        message: SubjectMessage,
    ) -> anyhow::Result<()> {
        let (instance, mut store) = self.engine.prepare_instance(&config.component).await?;
        let engine = wit::messages::Messages::new(&mut store, &instance, |data| data.as_mut())?;

        let metadata = message
            .message
            .metadata
            .iter()
            .map(|v| MetadataParam {
                name: &v.name,
                value: v.value.as_slice(),
            })
            .collect::<Vec<_>>();

        let original_subject = &message.subject;

        let message = SubjectMessageParam {
            subject: &message.subject,
            message: MessageParam {
                body: message.message.body.as_deref(),
                metadata: metadata.as_slice(),
            },
        };

        let result = engine.handle_message(&mut store, message).await?;

        match (result, &config.result) {
            (Outcome::Response(msg), MessageResultType::Response { broker, subject }) => {
                let msg = SubjectMessage {
                    subject: subject.clone(),
                    message: msg.unwrap_or(Message { body: None, metadata: vec![]})
                };
                self.send_with_broker(broker, msg).await?;
            },
            (Outcome::Response(msg), MessageResultType::Publish { broker }) => {
                let msg = SubjectMessage {
                    subject: original_subject.clone(),
                    message: msg.unwrap_or(Message { body: None, metadata: vec![]})
                };
                self.send_with_broker(broker, msg).await?;
            },
            (Outcome::Publish(msgs), MessageResultType::Response { broker, subject: _ }) => self.send_all_with_broker(broker, msgs).await?,
            (Outcome::Publish(msgs), MessageResultType::Publish { broker }) => self.send_all_with_broker(broker, msgs).await?,
            (Outcome::ErrorResponse(msg), MessageResultType::Response { broker, subject }) => {
                let msg = SubjectMessage {
                    subject: subject.clone(),
                    message: msg.unwrap_or(Message { body: None, metadata: vec![]})
                };
                self.send_with_broker(broker, msg).await?;
            },
            (Outcome::ErrorResponse(msg), MessageResultType::Publish { broker }) => {
                let msg = SubjectMessage {
                    subject: original_subject.clone(),
                    message: msg.unwrap_or(Message { body: None, metadata: vec![]})
                };
                self.send_with_broker(broker, msg).await?;
            },
            _ => {}
        }
        Ok(())
    }
}
