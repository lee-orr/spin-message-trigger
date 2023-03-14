use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use spin_message_types::export::{messages::MetadataResult, Message, SubjectMessage};

use crate::broker::MessageBroker;

pub async fn spawn_gateway(port: u16, _websockets: bool, broker: Arc<dyn MessageBroker>) {
    let app = Router::new()
        .route("/publish/*subject", post(publish))
        .with_state(broker);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn publish(
    Path(subject): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(broker): State<Arc<dyn MessageBroker>>,
    body: Bytes,
) -> impl IntoResponse {
    match broker
        .publish(SubjectMessage {
            subject: Some(subject),
            message: Message {
                body: Some(body.into()),
                metadata: params
                    .iter()
                    .map(|(key, value)| MetadataResult {
                        name: key.clone(),
                        value: value.clone().into_bytes(),
                    })
                    .collect(),
            },
            broker: None,
        })
        .await
    {
        Ok(_) => (StatusCode::ACCEPTED, "published to subject"),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "couldn't publish"),
    }
}
