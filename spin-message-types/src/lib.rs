use std::fmt::Display;

use serde::{Serialize, Deserialize};
#[cfg(feature = "export")]
pub mod export;

#[cfg(feature = "import")]
#[allow(clippy::all)]
pub mod import;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Message {
    pub body: Option<Vec<u8>>,
    pub metadata: Vec<Metadata>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct SubjectMessage {
    pub message: Message,
    pub subject: Option<String>,
    pub broker: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageError(String);

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Message Processing Error: {}", self.0))
    }
}