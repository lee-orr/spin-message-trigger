use anyhow::bail;
use std::future::Future;

use crate::{Message, Metadata, SubjectMessage};

use self::messages::{InternalMessageResult, InternalMetadataResult, InternalSubjectMessageResult};

wit_bindgen_wasmtime::import!({paths: ["exported.wit"], async: *});

pub mod messages {
    pub use crate::export::exported::*;
}

impl From<InternalSubjectMessageResult> for SubjectMessage {
    fn from(value: InternalSubjectMessageResult) -> Self {
        Self {
            message: value.message.into(),
            subject: value.subject,
            broker: value.broker,
        }
    }
}

impl From<InternalMessageResult> for Message {
    fn from(value: InternalMessageResult) -> Self {
        Self {
            body: value.body,
            metadata: value.metadata.into_iter().map(|v| v.into()).collect(),
        }
    }
}

impl From<InternalMetadataResult> for Metadata {
    fn from(value: InternalMetadataResult) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

pub async fn call_with_params<
    Fut: Future<Output = anyhow::Result<messages::Outcome>>,
    T: FnOnce(messages::InternalSubjectMessageParam) -> Fut,
>(
    message: SubjectMessage,
    broker: &str,
    f: T,
) -> anyhow::Result<Vec<SubjectMessage>> {
    let metadata = message
        .message
        .metadata
        .iter()
        .map(|v| messages::InternalMetadataParam {
            name: &v.name,
            value: v.value.as_slice(),
        })
        .collect::<Vec<_>>();

    let message = messages::InternalSubjectMessageParam {
        subject: message.subject.as_deref(),
        message: messages::InternalMessageParam {
            body: message.message.body.as_deref(),
            metadata: metadata.as_slice(),
        },
        broker: Some(broker),
    };

    let outcome = f(message).await?;

    match outcome {
        messages::Outcome::Publish(vec) => Ok(vec.into_iter().map(|v| v.into()).collect()),
        messages::Outcome::Error(e) => {
            bail!(e)
        }
    }
}
