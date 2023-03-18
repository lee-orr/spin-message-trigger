use std::fmt::Display;

use http::{HeaderMap, Method, StatusCode, Uri};
use serde::{Deserialize, Serialize};
#[cfg(feature = "export")]
pub mod export;

#[cfg(feature = "import")]
#[allow(clippy::all)]
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
pub struct MessageError(String);

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
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    #[serde(with = "http_serde::status_code")]
    pub status: StatusCode,
    pub body: Vec<u8>,
}
