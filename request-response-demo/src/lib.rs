use http::StatusCode;
use spin_message_types::import::json_http_component;
use spin_message_types::*;
use std::str;

#[json_http_component]
fn handle_message(message: HttpRequest) -> HttpResponse {
    println!("Http Request: {message:?}");
    let Ok(body) =  str::from_utf8(&message.body) else {
        return HttpResponse {
            headers: Default::default(),
            status: StatusCode::BAD_REQUEST,
            body: vec![],
        };
    };
    let path = message.path;
    HttpResponse {
        headers: Default::default(),
        status: StatusCode::OK,
        body: format!("Path: {path}, Recieved: {body}").as_bytes().to_owned(),
    }
}
