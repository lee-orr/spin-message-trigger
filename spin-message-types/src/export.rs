wit_bindgen_wasmtime::import!({paths: ["exported.wit"], async: *});

pub mod messages {
    pub use crate::export::exported::*;
}

pub type SubjectMessage = crate::export::messages::SubjectMessageResult;
pub type Message = crate::export::messages::MessageResult;
