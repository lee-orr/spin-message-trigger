use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn message_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        
        #func

        mod inner_handle {
            use super::*;
            use spin_message_types::import::linkme::distributed_slice;
        extern crate linkme;

        #[distributed_slice(spin_message_types::import::HANDLE_MESSAGE)]
        pub static HANDLE_MESSAGE_STATIC_FN: fn(spin_message_types::import::InternalMessage) -> spin_message_types::import::Outcome = handle_message_generated_fn;
        
        fn handle_message_generated_fn(message: spin_message_types::import::InternalMessage) -> spin_message_types::import::Outcome  {
                let message : spin_message_types::InputMessage = message.into();

                let response_subject = message.response_subject.clone();

                let Ok(runtime) = spin_message_types::runtime::runtime() else {
                    return Result::<Vec<OutputMessage>, MessageError>::Err(MessageError("Couldn't generate runtime".to_string())).into();
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
        async fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
            #func

            if let Ok(http) = HttpRequest::from_json_message(&message) {
                let result : HttpResponse = #func_name(http).await;
                Ok(result.to_json_response())
            } else {
                Err(MessageError("Couldn't parse http request".to_string()))
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
