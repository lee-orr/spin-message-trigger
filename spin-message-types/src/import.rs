pub use message_macro::*;
pub use wit_bindgen_rust::*;

wit_bindgen::generate!({
    path: "wit/spin-message-trigger.wit",
    world: "spin-message-trigger",
    exports: { "guest": EntryPoint }
});

pub use config::{get_config, Error};
pub use self::exports::guest::*;
pub use self::leeorr::spin_message_trigger::spin_message_types::InternalOutputMessage;
use linkme::distributed_slice;
pub use linkme;


#[distributed_slice]
pub static HANDLE_MESSAGE: [fn(InternalMessage) -> Outcome];

pub struct EntryPoint;

impl crate::import::exports::guest::Guest for EntryPoint {
    fn handle_message(message:InternalMessage,) -> Outcome {
        match HANDLE_MESSAGE.first() {
            Some(f) => f(message),
            None => Outcome::Error("No Message Handler Defined".to_string()),
        }
    }
}


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
