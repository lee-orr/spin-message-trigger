use crate::{InputMessage, OutputMessage};

mod inner {
    wasmtime::component::bindgen!({
        path: "wit-message",
        async: true,
        world: "spin-message-trigger-guest",
    });
}

pub use self::inner::leeorr::spin_message_trigger::spin_message_types::{
    InternalMessage, InternalOutputMessage, Outcome,
};
pub use self::inner::SpinMessageTriggerGuest;

impl From<InternalOutputMessage> for OutputMessage {
    fn from(value: InternalOutputMessage) -> Self {
        Self {
            message: value.message,
            subject: value.subject,
            broker: value.broker,
            response_subject: value.response_subject,
        }
    }
}

impl From<InternalMessage> for InputMessage {
    fn from(value: InternalMessage) -> Self {
        Self {
            message: value.message.to_vec(),
            subject: value.subject.to_string(),
            broker: value.broker.to_string(),
            response_subject: value.response_subject.map(|a| a.to_owned()),
        }
    }
}
