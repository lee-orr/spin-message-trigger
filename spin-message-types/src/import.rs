pub use message_macro::*;
pub use wit_bindgen_rust::*;

wit_bindgen::generate!({
    path: "wit",
    world: "spin-message-trigger"
});

pub use config::{get_config, Error};
pub use self::leeorr::spin_message_trigger::spin_message_types::{InternalOutputMessage, InternalMessage, Outcome };
pub use self::leeorr::spin_message_trigger::spin_message_types as spin_message_types;


impl From<crate::InputMessage> for InternalMessage {
    fn from(value: crate::InputMessage) -> Self {
        Self {
            message: value.message,
            broker: value.broker,
            subject: value.subject,
            response_subject: value.response_subject,
        }
    }
}

impl From<InternalMessage> for crate::InputMessage {
    fn from(value: InternalMessage) -> Self {
        Self {
            message: value.message,
            broker: value.broker,
            subject: value.subject,
            response_subject: value.response_subject,
        }
    }
}

impl From<crate::OutputMessage> for InternalOutputMessage {
    fn from(value: crate::OutputMessage) -> Self {
        Self {
            message: value.message.into(),
            subject: value.subject,
            broker: value.broker,
            response_subject: value.response_subject,
        }
    }
}

impl From<InternalOutputMessage> for crate::OutputMessage {
    fn from(value: InternalOutputMessage) -> Self {
        Self {
            message: value.message.into(),
            subject: value.subject,
            broker: value.broker,
            response_subject: value.response_subject,
        }
    }
}

impl From<Result<Vec<crate::OutputMessage>, crate::MessageError>> for Outcome {
    fn from(value: Result<Vec<crate::OutputMessage>, crate::MessageError>) -> Self {
        match value {
            Ok(vec) => {
                Outcome::Publish(vec.into_iter().map(|v| v.into()).collect())
            }
            Err(err) => Outcome::Error(err.to_string()),
        }
    }
}
