use std::str::from_utf8;

use spin_message_types::import::message_component;
use spin_message_types::*;

#[message_component]
async fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
    let msg = from_utf8(&message.message);
    let output: Vec<u8> = format!("Received: {msg:?}").bytes().collect();
    Ok(vec![OutputMessage {
        message: output,
        ..Default::default()
    }])
}
