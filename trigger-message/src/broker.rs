use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::Serialize;
use tokio::sync::broadcast;

use spin_message_types::{HttpRequest, HttpResponse, InputMessage, OutputMessage};

use crate::configs::GatewayRequestResponseConfig;

pub type Receiver = broadcast::Receiver<InputMessage>;
pub type Sender = broadcast::Sender<InputMessage>;

pub fn create_channel(capacity: usize) -> Sender {
    let (sender, _) = broadcast::channel(capacity);
    sender
}

#[async_trait]
pub trait MessageBroker: Send + Sync {
    fn name(&self) -> &str;
    async fn publish(&self, message: OutputMessage) -> Result<()>;
    async fn subscribe(&self, subject: &str) -> Result<Receiver>;

    async fn publish_all(&self, messages: Vec<OutputMessage>) -> Result<()> {
        for msg in messages.into_iter() {
            self.publish(msg).await?;
        }
        Ok(())
    }

    fn generate_request_subjects(&self, path: &str, method: &http::Method) -> (String, String) {
        let request_id = ulid::Ulid::new();
        let path = path.replace('.', "_DOT_").replace('/', ".");
        let subject_base = format!("{request_id}.{method}.{path}");
        let subject = format!("request.{subject_base}");
        let response_subject = format!("response.{subject_base}");
        (subject, response_subject)
    }

    fn generate_request_subscription(&self, path: &str, method: &Option<String>) -> String {
        let method = method.clone().unwrap_or("*".to_string());
        let path = path.replace('.', "_DOT_").replace('/', ".");
        format!("request.*.{method}.{path}")
    }

    async fn request(
        &self,
        mut request: HttpRequest,
        serializer: &GatewayRequestResponseConfig,
    ) -> Result<HttpResponse> {
        let (subject, response_subject) =
            self.generate_request_subjects(&request.path, &request.method);

        request.request_subject = subject.clone();
        request.response_subject = response_subject.clone();

        let body = match serializer {
            GatewayRequestResponseConfig::Messagepack => {
                let mut buf = Vec::new();
                if let Ok(()) = request.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
                    Some(buf)
                } else {
                    None
                }
            }
            GatewayRequestResponseConfig::Json => {
                serde_json::to_string(&request).ok().map(|v| v.into_bytes())
            }
        };

        let (Some(body), Ok(mut subscribe)) = (body, self.subscribe(&response_subject).await) else {
            bail!("Couldn't Subscribe");
        };

        self.publish(OutputMessage {
            subject: Some(subject.clone()),
            message: body,
            broker: None,
        })
        .await?;

        let Ok(result) = subscribe.recv().await else {
            bail!("couldn't get result");
        };

        println!("Got Response: {result:?}");

        let result = match serializer {
            GatewayRequestResponseConfig::Messagepack => {
                rmp_serde::from_slice::<HttpResponse>(&result.message).ok()
            }
            GatewayRequestResponseConfig::Json => {
                serde_json::from_slice::<HttpResponse>(&result.message).ok()
            }
        };
        if let Some(result) = result {
            Ok(result)
        } else {
            bail!("couldn't process result")
        }
    }
}
