use axum::{
    body::Bytes,
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use spin_message_types::export::{messages::MetadataResult, Message, SubjectMessage};

use crate::broker::MessageBroker;

pub async fn spawn_gateway(port: u16, websockets: bool, broker: Arc<dyn MessageBroker>) {
    let app = Router::new()
        .route("/publish/*subject", post(publish))
        .route(
            "/subscribe/*subject",
            if websockets {
                get(subscribe)
            } else {
                get(cannot_subscribe)
            },
        )
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

async fn cannot_subscribe() -> impl IntoResponse {
    "Websocket Subscriptions Aren't Supported"
}

async fn subscribe(
    Path(subject): Path<String>,
    State(broker): State<Arc<dyn MessageBroker>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, subject, broker))
}

async fn handle_websocket(mut socket: WebSocket, subject: String, broker: Arc<dyn MessageBroker>) {
    if let Ok(mut result) = broker.subscribe(&subject) {
        while let Ok(message) = result.recv().await {
            if let Some(body) = message.message.body {
                let _ = socket.send(WsMessage::Binary(body)).await;
            }
        }
    }
}
