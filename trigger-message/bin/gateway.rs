use std::sync::Arc;

use anyhow::Error;
use clap::Parser;
use trigger_message::{
    broker::MessageBroker,
    configs::{BrokerTypeConfig, GatewayRequestResponseConfig, WebsocketConfig},
    gateway::spawn_gateway,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Websocket Protocol Type
    #[clap(short, long)]
    websockets: Option<WebsocketConfig>,

    /// Request Response Protocol
    #[clap(short, long)]
    request_response: Option<GatewayRequestResponseConfig>,

    /// Gateway Port
    #[clap(short, long, default_value_t = 3015)]
    port: u16,

    /// Gateway Timeout
    #[clap(short, long)]
    timeout: Option<u64>,

    /// A query string defining the broker
    #[clap(short, long)]
    broker: BrokerTypeConfig,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let Args {
        port,
        websockets,
        timeout,
        request_response,
        broker,
    } = Args::parse();

    let broker_key: String = "BROKER".to_string();

    let broker: Arc<dyn MessageBroker> =
        match broker {
            BrokerTypeConfig::InMemoryBroker => Arc::new(
                trigger_message::in_memory_broker::InMemoryBroker::new(broker_key.clone()),
            ),
            BrokerTypeConfig::Redis(address) => Arc::new(
                trigger_message::redis_broker::RedisBroker::new(address, broker_key.clone()),
            ),
            BrokerTypeConfig::Nats(options) => Arc::new(
                trigger_message::nats_broker::NatsBroker::new(options, broker_key.clone()),
            ),
            BrokerTypeConfig::Mqtt(options) => Arc::new(
                trigger_message::mqtt_broker::MqttBroker::new(options, broker_key.clone()),
            ),
        };

    spawn_gateway(port, websockets.clone(), broker, request_response, timeout).await;

    Ok(())
}
