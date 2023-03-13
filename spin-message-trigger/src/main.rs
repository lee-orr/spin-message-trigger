mod wit;
mod broker;
mod in_memory_broker;

use serde::{Deserialize, Serialize};
use spin_trigger::{TriggerAppEngine, TriggerExecutor};



fn main() {
    println!("Hello, world!");
}

type RuntimeData = wit::messages::MessagesData;

struct MessageTrigger {
    engine: TriggerAppEngine<Self>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MessageMetadata {

}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MessageTriggerConfig {

}

#[async_trait::async_trait]
impl TriggerExecutor for MessageTrigger {
    const TRIGGER_TYPE: &'static str = "message";

    type RuntimeData = RuntimeData;

    type TriggerConfig = MessageTriggerConfig;

    type RunConfig = spin_trigger::cli::NoArgs;

    fn new(engine:TriggerAppEngine<Self>) -> anyhow::Result<Self>  {
        todo!()
    }

    async fn run(self,config:Self::RunConfig) ->  anyhow::Result<()>{
        todo!()
    }
}