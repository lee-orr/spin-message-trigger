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
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};

use spin_message_types::{HttpRequest, OutputMessage, HttpResponse};

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
        .route("/ws", any(ws_handler))
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
            handle_subscribe_websocket(socket, subject, state.broker.clone(), websockets)
        })
        .into_response()
    } else {
        (StatusCode::BAD_REQUEST, "Websockets aren't supported").into_response()
    }
}

async fn handle_subscribe_websocket(
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

fn axum_to_http(method: axum::http::Method, headers: axum::http::HeaderMap, uri: axum::http::Uri, path: String, bytes: Bytes) -> anyhow::Result<HttpRequest> {
    let mut headers_iter = headers.into_iter().map(|(n,v)| (n.and_then(|n| str::parse::<http::HeaderName>(&n.to_string()).ok()), http::HeaderValue::from_bytes(v.as_bytes()))).filter_map(|(n, v)| match v {
        Ok(v) => Some((n,v)),
        Err(_) => None,
    });

    let mut headers = http::HeaderMap::new();

    headers.extend(headers_iter);

    let request = HttpRequest {
        method: str::parse(&method.to_string())?,
        headers,
        uri: str::parse(&uri.to_string())?,
        path: path.clone(),
        body: bytes.to_vec(),
    };
    Ok(request)
}



fn http_to_axum(code: http::StatusCode, headers: http::HeaderMap,body: Vec<u8>) -> anyhow::Result<Response<BoxBody>> {
    let mut headers_iter = headers.into_iter().map(|(n,v)| (n.and_then(|n| str::parse::<axum::http::HeaderName>(&n.to_string()).ok()), axum::http::HeaderValue::from_bytes(v.as_bytes()))).filter_map(|(n, v)| match v {
        Ok(v) => Some((n,v)),
        Err(_) => None,
    });

    let mut headers = axum::http::HeaderMap::new();

    headers.extend(headers_iter);

    let response = (StatusCode::from_u16(code.as_u16())?, headers, body).into_response();

    Ok(response)
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

        let Ok(request) = axum_to_http(method, headers, uri, path, bytes) else {
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        };
        match tokio::time::timeout(timeout, broker.http_request(request, serializer)).await {
            Ok(Ok(result)) => {
                println!("Got Parsed Response: {result:?}");
                match http_to_axum(result.status, result.headers, result.body) {
                    Ok(r) => r,
                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
                }
            }
            Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            _ => (StatusCode::GATEWAY_TIMEOUT, "response timed out").into_response(),
        }
    } else {
        (StatusCode::BAD_REQUEST, "request-response is not supported").into_response()
    }
}

async fn ws_handler(
    State(state): State<Arc<GatewayState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    println!("Setting up upgrade");
    let websockets = state.websockets.clone();

    if let Some(websockets) = websockets {
        ws.on_upgrade(move |socket| {
            handle_bidirectional_websocket(socket, state.broker.clone(), websockets)
        })
        .into_response()
    } else {
        (StatusCode::BAD_REQUEST, "Websockets aren't supported").into_response()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum BidirectionalSocketMessage {
    Subscribe(String),
    Publish(OutputMessage),
}

async fn handle_bidirectional_websocket(
    socket: WebSocket,
    broker: Arc<dyn MessageBroker>,
    websockets: configs::WebsocketConfig,
) {
    println!("upgraded");
    let (mut sender, receiver) = socket.split();

    let broker = broker.clone();
    let (channel_send, mut channel_recv) = tokio::sync::mpsc::channel(10);
    tokio::spawn(async move {
        receive_websocket_messages(receiver, &broker, channel_send, websockets).await
    });

    while let Some(t) = channel_recv.recv().await {
        let _ = sender.send(t).await;
    }
}

async fn websocket_subscribe(
    broker: Arc<dyn MessageBroker>,
    subject: String,
    websockets: configs::WebsocketConfig,
    sender: tokio::sync::mpsc::Sender<WsMessage>,
) {
    let is_binary = match websockets {
        configs::WebsocketConfig::BinaryBody => true,
        configs::WebsocketConfig::TextBody => false,
        configs::WebsocketConfig::Messagepack => true,
        configs::WebsocketConfig::Json => false,
    };

    if let Ok(mut result) = broker.subscribe_to_topic(&subject).await {
        println!("subscribed to {subject}");
        while let Ok(message) = result.recv().await {
            println!("socket subscription message recieved");
            match is_binary {
                true => {
                    let mut buf = Vec::new();
                    if let Ok(()) = message.serialize(&mut rmp_serde::Serializer::new(&mut buf)) {
                        let _ = sender.send(WsMessage::Binary(buf)).await;
                        println!("socket subscription messagepack sent");
                    }
                }
                false => {
                    if let Ok(json) = serde_json::to_string(&message) {
                        let _ = sender.send(WsMessage::Text(json)).await;
                        println!("socket subscription message json sent");
                    }
                }
            }
        }
    }
}

async fn receive_websocket_messages(
    mut receiver: futures::stream::SplitStream<WebSocket>,
    broker: &Arc<dyn MessageBroker>,
    sender: tokio::sync::mpsc::Sender<WsMessage>,
    websocket: configs::WebsocketConfig,
) {
    loop {
        println!("waiting for message");
        match receiver.next().await {
            Some(Ok(msg)) => {
                println!("parsing...");
                let Some(parsed) = (match msg {
                    WsMessage::Text(text) => serde_json::from_str(&text).ok(),
                    WsMessage::Binary(binary) => rmp_serde::from_slice(&binary).ok(),
                    WsMessage::Close(reason) => {
                        println!("Websocket closed {reason:?}");
                        break;
                    },
                    _ => { continue; }
                 }) else { println!("parse failed"); break; };

                match parsed {
                    BidirectionalSocketMessage::Subscribe(subject) => {
                        let broker = broker.clone();
                        let websocket = websocket.clone();
                        let sender = sender.clone();
                        tokio::spawn(async move {
                            websocket_subscribe(broker, subject, websocket, sender).await;
                        });
                    }
                    BidirectionalSocketMessage::Publish(message) => {
                        println!("publishing to broker {message:?}");
                        let _ = broker.publish(message).await;
                    }
                }
            }
            Some(Err(e)) => {
                eprintln!("Error receiving socket {e:?}");
                break;
            }
            None => {
                eprintln!("Received nothing...");
                break;
            }
        };
        println!("looping back");
    }
    println!("exited");
}
