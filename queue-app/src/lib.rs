use std::str::from_utf8;

use spin_message_types::import::{config, message_component};
use spin_message_types::*;

#[message_component]
fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
    let queue_id = config::get_config("queue_id");
    println!(
        "Process Queue ({queue_id:?}) Message: {:?}",
        from_utf8(&message.message)
    );
    Ok(vec![])
}
