use proc_macro::TokenStream;
use quote::quote;

const INLINE_WIT : &'static str = include_str!("../../wit/spin-message-trigger.wit");

#[proc_macro_attribute]
pub fn message_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        
        #func

        mod inner_handle {
            use super::#func_name;

        wit_bindgen::generate!({
            inline: #INLINE_WIT,
            world: "spin-message-trigger-guest",
            exports: { "guest": MessageHandler},
        });

        use self::exports::guest::Guest;
        use self::leeorr::spin_message_trigger::spin_message_types::{InternalMessage, InternalOutputMessage, Outcome};

        impl From<spin_message_types::InputMessage> for InternalMessage {
            fn from(value: spin_message_types::InputMessage) -> Self {
                Self {
                    message: value.message,
                    broker: value.broker,
                    subject: value.subject,
                    response_subject: value.response_subject,
                }
            }
        }
        
        impl From<InternalMessage> for spin_message_types::InputMessage {
            fn from(value: InternalMessage) -> Self {
                Self {
                    message: value.message,
                    broker: value.broker,
                    subject: value.subject,
                    response_subject: value.response_subject,
                }
            }
        }
        
        impl From<spin_message_types::OutputMessage> for InternalOutputMessage {
            fn from(value: spin_message_types::OutputMessage) -> Self {
                Self {
                    message: value.message.into(),
                    subject: value.subject,
                    broker: value.broker,
                    response_subject: value.response_subject,
                }
            }
        }
        
        impl From<InternalOutputMessage> for spin_message_types::OutputMessage {
            fn from(value: InternalOutputMessage) -> Self {
                Self {
                    message: value.message.into(),
                    subject: value.subject,
                    broker: value.broker,
                    response_subject: value.response_subject,
                }
            }
        }
        
        impl From<Result<Vec<spin_message_types::OutputMessage>, spin_message_types::MessageError>> for Outcome {
            fn from(value: Result<Vec<spin_message_types::OutputMessage>, spin_message_types::MessageError>) -> Self {
                match value {
                    Ok(vec) => {
                        Outcome::Publish(vec.into_iter().map(|v| v.into()).collect())
                    }
                    Err(err) => Outcome::Error(err.to_string()),
                }
            }
        }

        struct MessageHandler;

        impl Guest for MessageHandler {
            fn handle_message(message: InternalMessage) -> Outcome  {
                let message : spin_message_types::InputMessage = message.into();

                let response_subject = message.response_subject.clone();

                let Ok(runtime) = spin_message_types::runtime::runtime() else {
                    return Result::<Vec<spin_message_types::OutputMessage>, spin_message_types::MessageError>::Err(spin_message_types::MessageError("Couldn't generate runtime".to_string())).into();
                };

                let mut result = runtime.block_on(async {
                    let mut result = #func_name(message);
                    let output = result.await;
                    output
                });

                println!("Responding with {:?}", response_subject);

                if let Ok(mut v) = result.as_mut() {
                    for mut msg in v.iter_mut() {
                        if msg.subject.is_none() {
                            if let Some(response) = &response_subject {
                                let _ = msg.subject.insert(response.clone());
                            }
                        }
                    }
                }

                result.into()
            }
        }    
    }
    )
    .into()
}

#[proc_macro_attribute]
pub fn json_http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        use spin_message_types::import::message_component;

        #[message_component]
        async fn handle_message(message: spin_message_types::InputMessage) -> Result<Vec<spin_message_types::OutputMessage>, spin_message_types::MessageError> {
            #func

            if let Ok(http) = HttpRequest::from_json_message(&message) {
                let result : HttpResponse = #func_name(http).await;
                Ok(result.to_json_response())
            } else {
                Err(spin_message_types::MessageError("Couldn't parse http request".to_string()))
            }
        }
    )
    .into()
}

#[proc_macro_attribute]
pub fn msgpack_http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        use spin_message_types::import::message_component;

        #[message_component]
        async fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
            #func

            if let Ok(http) = HttpRequest::from_msgpack_message(&message) {
                let result : HttpResponse = #func_name(http).await;
                Ok(result.to_msgpack_response())
            } else {
                Err(MessageError("Couldn't parse http request".to_string()))
            }
        }
    )
    .into()
}
