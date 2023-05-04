use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn message_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        struct Messages;

        impl spin_message_types::import::guest::Guest for Messages {
            fn handle_message(message: spin_message_types::import::spin_message_types::InternalMessage) -> spin_message_types::import::spin_message_types::Outcome {
                let message : spin_message_types::InputMessage = message.into();
                #func

                #func_name(message).into()
            }
        }

        const _: () = {
            #[doc(hidden)]
            #[export_name = "guest#handle-message"]
            #[allow(non_snake_case)]
            unsafe extern "C" fn __export_guest_handle_message(arg0: i32,arg1: i32,arg2: i32,arg3: i32,arg4: i32,arg5: i32,) -> i32 {
                spin_message_types::import::guest::call_handle_message::<Messages>(arg0,arg1,arg2,arg3,arg4,arg5,)
            }

            #[doc(hidden)]
            #[export_name = "cabi_post_guest#handle-message"]
            #[allow(non_snake_case)]
            unsafe extern "C" fn __post_return_guest_handle_message(arg0: i32,) {
                spin_message_types::import::guest::post_return_handle_message::<Messages>(arg0,)
            }
        };
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
        fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
            #func

            if let Ok(http) = HttpRequest::from_json_message(&message) {
                let result : HttpResponse = #func_name(http);
                Ok(result.to_json_response(message.subject.as_str()))
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
        fn handle_message(message: InputMessage) -> Result<Vec<OutputMessage>, MessageError> {
            #func

            if let Ok(http) = HttpRequest::from_msgpack_message(&message) {
                let result : HttpResponse = #func_name(http);
                Ok(result.to_msgpack_response(message.subject.as_str()))
            } else {
                Err(MessageError("Couldn't parse http request".to_string()))
            }
        }
    )
    .into()
}
