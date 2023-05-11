use axum::{
    body::{BoxBody, Bytes},
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{HeaderMap, Method, Response, StatusCode, Uri},
    response::IntoResponse,
    routing::{any, get, post},
    Router,
};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use spin_message_types::{HttpRequest, OutputMessage};

use crate::{
    broker::MessageBroker,
    configs::{self, GatewayRequestResponseConfig},
};

#[derive(Clone)]
struct GatewayState {
    broker: Arc<dyn MessageBroker>,
    websockets: Option<configs::WebsocketConfig>,
    request_response: Option<configs::GatewayRequestResponseConfig>,
    timeout: Option<u64>,
}

pub async fn spawn_gateway(
    port: u16,
    websockets: Option<configs::WebsocketConfig>,
    broker: Arc<dyn MessageBroker>,
    request_response: Option<GatewayRequestResponseConfig>,
    timeout: Option<u64>,
) {
    let app = Router::new()
        .route("/publish/*subject", post(publish))
        .route("/subscribe/*subject", get(subscribe))
        .route("/request/*path", any(request_handler))
        .with_state(Arc::new(GatewayState {
            broker,
            websockets,
            request_response,
            timeout,
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn publish(
    Path(subject): Path<String>,
    State(state): State<Arc<GatewayState>>,
    body: Bytes,
) -> impl IntoResponse {
    let broker = &state.broker;
    match broker
        .publish(OutputMessage {
            subject: Some(subject),
            message: body.to_vec(),
            broker: None,
            response_subject: None,
        })
        .await
    {
        Ok(_) => (StatusCode::ACCEPTED, "published to subject"),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "couldn't publish"),
    }
}

async fn subscribe(
    Path(subject): Path<String>,
    State(state): State<Arc<GatewayState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    println!("Setting up upgrade");
    let websockets = state.websockets.clone();

    if let Some(websockets) = websockets {
        ws.on_upgrade(move |socket| {
            handle_websocket(socket, subject, state.broker.clone(), websockets)
        })
        .into_response()
    } else {
        (StatusCode::BAD_REQUEST, "Websockets aren't supported").into_response()
    }
}

async fn handle_websocket(
    mut socket: WebSocket,
    subject: String,
    broker: Arc<dyn MessageBroker>,
    websockets: configs::WebsocketConfig,
) {
    println!("upgraded");
    if let Ok(mut result) = broker.subscribe_to_topic(&subject).await {
        println!("subscribed to {subject}");
        while let Ok(message) = result.recv().await {
            println!("socket subscription message recieved");
            match websockets {
                configs::WebsocketConfig::BinaryBody => {
                    let _ = socket.send(WsMessage::Binary(message.message)).await;
                    println!("socket subscription message sent");
                }
                configs::WebsocketConfig::TextBody => {
                    if let Ok(body) = std::str::from_utf8(&message.message) {
                        let _ = socket.send(WsMessage::Text(body.to_string())).await;
                        println!("socket subscription message sent");
                    }
                }
                configs::WebsocketConfig::Messagepack => {
                    let mut buf = Vec::new();
                    if let Ok(()) = message.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
                        let _ = socket.send(WsMessage::Binary(buf)).await;
                        println!("socket subscription messagepack sent");
                    }
                }
                configs::WebsocketConfig::Json => {
                    if let Ok(json) = serde_json::to_string(&message) {
                        let _ = socket.send(WsMessage::Text(json)).await;
                        println!("socket subscription message json sent");
                    }
                }
            }
        }
    }
}

async fn request_handler(
    Path(path): Path<String>,
    State(state): State<Arc<GatewayState>>,
    uri: Uri,
    method: Method,
    headers: HeaderMap,
    bytes: Bytes,
) -> Response<BoxBody> {
    if let Some(serializer) = &state.request_response {
        let broker = &state.broker;
        let timeout = state.timeout.unwrap_or(2000);
        let timeout = Duration::from_millis(timeout);

        let request = HttpRequest {
            method: method.clone(),
            headers: headers.clone(),
            uri: uri.clone(),
            path: path.clone(),
            body: bytes.to_vec(),
        };
        match tokio::time::timeout(timeout, broker.http_request(request, serializer)).await {
            Ok(Ok(result)) => {
                println!("Got Parsed Response: {result:?}");
                (result.status, result.headers, result.body).into_response()
            }
            Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            _ => (StatusCode::GATEWAY_TIMEOUT, "response timed out").into_response(),
        }
    } else {
        (StatusCode::BAD_REQUEST, "request-response is not supported").into_response()
    }
}
