use crate::{InputMessage, OutputMessage};

wasmtime::component::bindgen!({
    path: "wit/spin-message-trigger.wit",
    async: true
});

pub use self::spin_message_types::{InternalMessage, InternalOutputMessage, Outcome};

impl From<InternalOutputMessage> for OutputMessage {
    fn from(value: InternalOutputMessage) -> Self {
        Self {
            message: value.message,
            subject: value.subject,
            broker: value.broker,
        }
    }
}

impl<'a> From<InternalMessage<'a>> for InputMessage {
    fn from(value: InternalMessage<'a>) -> Self {
        Self {
            message: value.message.to_vec(),
            subject: value.subject.to_string(),
            broker: value.broker.to_string(),
        }
    }
}
