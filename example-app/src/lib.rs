use std::str::from_utf8;

use spin_message_types::import::{message_component};
use spin_message_types::*;

#[message_component]
fn handle_message(
    message: SubjectMessage,
) -> Result<Vec<SubjectMessage>, MessageError> {
    println!("got here");
    if let Some(body) = message.message.body {
        println!("{:?}", from_utf8(&body));
    }
    let output: Vec<u8> = "Goodbye".bytes().collect();
    Ok(vec![
        SubjectMessage {
            message: Message {
                body: Some(output),
                ..Default::default()
            },
            ..Default::default()
        }
    ])
}