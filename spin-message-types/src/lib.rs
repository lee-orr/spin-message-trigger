use anyhow::{bail, Result};
use http::{HeaderMap, Method, StatusCode, Uri};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[cfg(feature = "export")]
pub mod export;

#[cfg(feature = "import")]
#[allow(clippy::all)]
#[allow(unused_macros)]
pub mod import;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct InputMessage {
    pub message: Vec<u8>,
    pub subject: String,
    pub broker: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct OutputMessage {
    pub message: Vec<u8>,
    pub subject: Option<String>,
    pub broker: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageError(pub String);

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Message Processing Error: {}", self.0))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    #[serde(with = "http_serde::method")]
    pub method: Method,
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(with = "http_serde::uri")]
    pub uri: Uri,
    pub path: String,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn from_json_message(message: &InputMessage) -> Result<Self> {
        match serde_json::from_slice(&message.message) {
            Ok(result) => Ok(result),
            Err(_) => bail!("Couldn't deserialize message"),
        }
    }
    pub fn from_msgpack_message(message: &InputMessage) -> Result<Self> {
        match rmp_serde::from_slice(&message.message) {
            Ok(result) => Ok(result),
            Err(_) => bail!("Couldn't deserialize message"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(with = "http_serde::status_code")]
    pub status: StatusCode,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn to_json_response(&self, subject: &str) -> Vec<OutputMessage> {
        match serde_json::to_string(&self) {
            Ok(value) => {
                println!("Setting response json {value}");
                vec![OutputMessage {
                    message: value.into_bytes(),
                    subject: Some(subject.replace("request.", "response.")),
                    ..Default::default()
                }]
            }
            Err(_) => vec![],
        }
    }
    pub fn to_msgpack_response(&self, subject: &str) -> Vec<OutputMessage> {
        let mut buf = Vec::new();
        match self.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
            Ok(_) => vec![OutputMessage {
                message: buf,
                subject: Some(subject.replace("request.", "response.")),
                ..Default::default()
            }],
            Err(_) => vec![],
        }
    }
}
