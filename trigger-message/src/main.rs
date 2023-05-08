use anyhow::Error;
use clap::Parser;
use trigger_message::message_trigger::*;
use spin_trigger::cli::TriggerExecutorCommand;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Setting up Message Trigger");
    let t = Command::parse();
    t.run().await
}
