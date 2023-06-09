use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::Serialize;
use tokio::sync::broadcast;

use spin_message_types::{HttpRequest, HttpResponse, InputMessage, OutputMessage};

use crate::configs::{GatewayRequestResponseConfig, SubscriptionType};

pub type Receiver = broadcast::Receiver<InputMessage>;
pub type QueueReceiver = broadcast::Receiver<InputMessage>;
pub type Sender = broadcast::Sender<InputMessage>;
pub type QueueSender = broadcast::Sender<InputMessage>;

pub fn create_channel(capacity: usize) -> Sender {
    let (sender, _) = broadcast::channel(capacity);
    sender
}

pub fn default_message_response_subject(subject: &str) -> Option<String> {
    if subject.starts_with("request") {
        Some(subject.replace("request", "response"))
    } else {
        None
    }
}

#[async_trait]
pub trait MessageBroker: Send + Sync {
    fn name(&self) -> &str;
    async fn publish(&self, message: OutputMessage) -> Result<()>;
    async fn subscribe_to_topic(&self, subject: &str) -> Result<Receiver>;

    async fn publish_all(&self, messages: Vec<OutputMessage>) -> Result<()> {
        for msg in messages.into_iter() {
            self.publish(msg).await?;
        }
        Ok(())
    }

    async fn subscribe_to_request(&self, path: &str, method: &Option<String>) -> Result<Receiver> {
        let method = method.clone().unwrap_or("*".to_string());
        let path = path.replace('.', "_DOT_").replace('/', ".");
        let sub = format!("request.*.{method}.{path}");
        self.subscribe_to_topic(&sub).await
    }

    async fn subscribe_to_queue(&self, topic: &str, group: &str) -> Result<QueueReceiver>;

    async fn subscribe(&self, subscription: &SubscriptionType) -> Result<Receiver> {
        match subscription {
            SubscriptionType::Topic { topic, result: _ } => self.subscribe_to_topic(topic).await,
            SubscriptionType::Request { path, method } => {
                self.subscribe_to_request(path, method).await
            }
            SubscriptionType::Queue {
                topic,
                group,
                result: _,
            } => self.subscribe_to_queue(topic, group).await,
            SubscriptionType::None => bail!("No Subscription Type Set for {}", self.name()),
        }
    }

    fn generate_http_request_subjects(
        &self,
        path: &str,
        method: &http::Method,
    ) -> (String, String) {
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

    async fn request(&self, mut request: OutputMessage) -> Result<InputMessage> {
        let Some(subject) = request.subject.clone() else {
            bail!("No subject set");
        };
        let response_subject = if let Some(resp) = &request.response_subject {
            resp.clone()
        } else {
            let resp = ulid::Ulid::new().to_string();
            let req = format!("request.{resp}.{subject}");
            let resp = format!("response.{resp}.{subject}");
            request.subject = Some(req);
            request.response_subject = Some(resp.clone());

            resp
        };

        let Ok(mut subscribe) = self.subscribe_to_topic(&response_subject).await else {
            bail!("Couldn't Subscribe");
        };

        self.publish(request).await?;

        let Ok(result) = subscribe.recv().await else {
            bail!("couldn't get result");
        };

        Ok(result)
    }

    async fn http_request(
        &self,
        request: HttpRequest,
        serializer: &GatewayRequestResponseConfig,
    ) -> Result<HttpResponse> {
        let (subject, response_subject) =
            self.generate_http_request_subjects(&request.path, &request.method);

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

        let Some(body) = body else {
            bail!("Couldn't Serialize body");
        };

        let message = OutputMessage {
            subject: Some(subject.clone()),
            message: body,
            broker: None,
            response_subject: Some(response_subject.clone()),
        };

        println!("Generated HTTP Request: {message:?}");

        let result = self.request(message).await?;

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
