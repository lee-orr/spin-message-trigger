use std::str::from_utf8;

use spin_message_types::import::message_component;
use spin_message_types::*;

#[message_component]
fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
    let queue_id = spin_sdk::config::get("queue_id");
    println!("Process Queue ({queue_id:?}) Message: {:?}", from_utf8(&message.message));
    Ok(vec![])
}
