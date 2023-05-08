use anyhow::Error;
use clap::Parser;
use trigger_message::message_trigger::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("Setting up Message Trigger");
    let t = Command::parse();
    t.run().await
}
