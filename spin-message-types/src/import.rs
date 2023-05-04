pub use message_macro::*;
pub use wit_bindgen_rust::*;

wit_bindgen::generate!({
    path: "wit/spin-message-trigger.wit",
    world: "spin-message-trigger"
});

impl From<crate::InputMessage> for spin_message_types::InternalMessage {
    fn from(value: crate::InputMessage) -> Self {
        Self {
            message: value.message,
            broker: value.broker,
            subject: value.subject,
        }
    }
}

impl From<spin_message_types::InternalMessage> for crate::InputMessage {
    fn from(value: spin_message_types::InternalMessage) -> Self {
        Self {
            message: value.message,
            broker: value.broker,
            subject: value.subject,
        }
    }
}

impl From<crate::OutputMessage> for spin_message_types::InternalOutputMessage {
    fn from(value: crate::OutputMessage) -> Self {
        Self {
            message: value.message.into(),
            subject: value.subject,
            broker: value.broker,
        }
    }
}

impl From<spin_message_types::InternalOutputMessage> for crate::OutputMessage {
    fn from(value: spin_message_types::InternalOutputMessage) -> Self {
        Self {
            message: value.message.into(),
            subject: value.subject,
            broker: value.broker,
        }
    }
}

impl From<Result<Vec<crate::OutputMessage>, crate::MessageError>> for spin_message_types::Outcome {
    fn from(value: Result<Vec<crate::OutputMessage>, crate::MessageError>) -> Self {
        match value {
            Ok(vec) => {
                spin_message_types::Outcome::Publish(vec.into_iter().map(|v| v.into()).collect())
            }
            Err(err) => spin_message_types::Outcome::Error(err.to_string()),
        }
    }
}