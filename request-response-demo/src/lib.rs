use http::StatusCode;
use spin_message_types::import::json_http_component;
use spin_message_types::*;

#[json_http_component]
fn handle_message(message: HttpRequest) -> HttpResponse {
    println!("Http Request: {message:?}");
    HttpResponse {
        headers: Default::default(),
        status: StatusCode::OK,
        body: "Recieved & Responded!".as_bytes().to_owned(),
    }
}
