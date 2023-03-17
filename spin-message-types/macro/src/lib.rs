use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn message_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    const MESSAGE_COMPONENT_WIT: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../exported.wit"));

    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        wit_bindgen_rust::export!({src["messages"]: #MESSAGE_COMPONENT_WIT});

        struct Messages;

        impl From<spin_message_types::InputMessage> for messages::InternalMessage {
            fn from(value: spin_message_types::InputMessage) -> Self {
                Self {
                    message: value.message,
                    broker: value.broker,
                    subject: value.subject
                }
            }
        }

        impl From<messages::InternalMessage> for spin_message_types::InputMessage {
            fn from(value: messages::InternalMessage) -> Self {
                Self {
                    message: value.message,
                    broker: value.broker,
                    subject: value.subject
                }
            }
        }

        impl From<spin_message_types::OutputMessage> for messages::InternalOutputMessage {
            fn from(value: spin_message_types::OutputMessage) -> Self {
                Self {
                    message: value.message.into(),
                    subject: value.subject,
                    broker: value.broker,
                }
            }
        }

        impl From<messages::InternalOutputMessage> for spin_message_types::OutputMessage {
            fn from(value: messages::InternalOutputMessage) -> Self {
                Self {
                    message: value.message.into(),
                    subject: value.subject,
                    broker: value.broker,
                }
            }
        }

        impl From<Result<Vec<spin_message_types::OutputMessage>,spin_message_types::MessageError>> for messages::Outcome {
            fn from(value: Result<Vec<spin_message_types::OutputMessage>,spin_message_types::MessageError>) -> Self {
                match value {
                    Ok(vec) => messages::Outcome::Publish(vec.into_iter().map(|v| v.into()).collect()),
                    Err(err) => messages::Outcome::Error(err.to_string())
                }
            }
        }

        impl messages::Messages for Messages {
            fn handle_message(message: messages::InternalMessage) -> messages::Outcome {
                let message : spin_message_types::InputMessage = message.into();
                #func

                #func_name(message).into()
            }
        }

    )
    .into()
}
