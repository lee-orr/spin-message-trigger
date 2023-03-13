wit_bindgen_wasmtime::import!({paths: ["messages.wit"], async: *});

pub type SubjectMessage = crate::wit::messages::SubjectMessageResult;
pub type Message = crate::wit::messages::MessageResult;