use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::nats_broker::NatsConnectionInfo;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BrokerConfig {
    pub broker_type: BrokerTypeConfig,
    pub gateway: GatewayConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum BrokerTypeConfig {
    #[default]
    InMemoryBroker,
    Redis(String),
    Nats(NatsConnectionInfo),
}

impl FromStr for BrokerTypeConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = serde_qs::from_str(s)?;
        Ok(result)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum WebsocketConfig {
    #[default]
    BinaryBody,
    TextBody,
    Messagepack,
    Json,
}

impl FromStr for WebsocketConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "messagepack" => Ok(WebsocketConfig::Messagepack),
            "json" => Ok(WebsocketConfig::Json),
            "binary" => Ok(WebsocketConfig::BinaryBody),
            "text" => Ok(WebsocketConfig::TextBody),
            _ => Err(anyhow::Error::msg("Invalid Websocket Protocol")),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum GatewayRequestResponseConfig {
    #[default]
    Messagepack,
    Json,
}

impl FromStr for GatewayRequestResponseConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "messagepack" => Ok(GatewayRequestResponseConfig::Messagepack),
            "json" => Ok(GatewayRequestResponseConfig::Json),
            _ => Err(anyhow::Error::msg(
                "Invalid Gateway Request Response Protocol",
            )),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum GatewayConfig {
    #[default]
    None,
    Http {
        port: u16,
        websockets: Option<WebsocketConfig>,
        request_response: Option<GatewayRequestResponseConfig>,
        timeout: Option<u64>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum SubscriptionType {
    #[default]
    None,
    Topic {
        topic: String,
        result: Option<MessageResultType>,
    },
    Request {
        path: String,
        method: Option<String>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageTriggerConfig {
    pub(crate) component: String,
    pub(crate) broker: String,
    pub(crate) subscription: SubscriptionType,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MessageResultType {
    pub(crate) default_broker: String,
    pub(crate) default_subject: String,
}
