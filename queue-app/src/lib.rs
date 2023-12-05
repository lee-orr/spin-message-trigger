use std::str::from_utf8;

use spin_message_types::import::{message_component, spin::variables};
use spin_message_types::*;

#[message_component]
async fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
    let queue_id = variables::get("queue_id");
    println!(
        "Process Queue ({queue_id:?}) Message: {:?}",
        from_utf8(&message.message)
    );
    Ok(vec![])
}
