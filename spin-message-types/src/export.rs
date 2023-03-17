use crate::{InputMessage, OutputMessage};

use self::messages::{InternalMessage, InternalOutputMessage};

wit_bindgen_wasmtime::import!({paths: ["exported.wit"], async: *});

pub mod messages {
    pub use crate::export::exported::*;
}

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
