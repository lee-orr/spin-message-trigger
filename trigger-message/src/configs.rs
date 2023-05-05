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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum WebsocketConfig {
    #[default]
    BinaryBody,
    TextBody,
    Messagepack,
    Json,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum GatewayRequestResponseConfig {
    #[default]
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
        request_response: Option<GatewayRequestResponseConfig>,
        timeout: Option<u64>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageTriggerConfig {
    pub(crate) component: String,
    pub(crate) broker: String,
    pub(crate) subscription: String,
    pub(crate) result: Option<MessageResultType>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MessageResultType {
    pub(crate) default_broker: String,
    pub(crate) default_subject: String,
}
