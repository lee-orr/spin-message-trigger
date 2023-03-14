use std::str::from_utf8;

struct Handler;

impl spin_message_types::import::Messages for Handler {
    fn handle_message(
        message: spin_message_types::import::SubjectMessage,
    ) -> spin_message_types::import::Outcome {
        println!("got here");
        if let Some(body) = message.message.body {
            println!("{:?}", from_utf8(&body));
        }
        spin_message_types::import::Outcome::Publish(vec![])
    }
}
