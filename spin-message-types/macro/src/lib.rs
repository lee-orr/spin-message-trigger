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

        impl From<spin_message_types::Metadata> for messages::InternalMetadata {
            fn from(value: spin_message_types::Metadata) -> Self {
                Self {
                    name: value.name,
                    value: value.value,
                }
            }
        }

        impl From<messages::InternalMetadata> for spin_message_types::Metadata {
            fn from(value: messages::InternalMetadata) -> Self {
                Self {
                    name: value.name,
                    value: value.value,
                }
            }

        }

        impl From<spin_message_types::Message> for messages::InternalMessage {
            fn from(value: spin_message_types::Message) -> Self {
                Self {
                    body: value.body,
                    metadata: value.metadata.into_iter().map(|v| v.into()).collect(),
                }
            }
        }

        impl From<messages::InternalMessage> for spin_message_types::Message {
            fn from(value: messages::InternalMessage) -> Self {
                Self {
                    body: value.body,
                    metadata: value.metadata.into_iter().map(|v| v.into()).collect(),
                }
            }
        }

        impl From<spin_message_types::SubjectMessage> for messages::InternalSubjectMessage {
            fn from(value: spin_message_types::SubjectMessage) -> Self {
                Self {
                    message: value.message.into(),
                    subject: value.subject,
                    broker: value.broker,
                }
            }
        }

        impl From<messages::InternalSubjectMessage> for spin_message_types::SubjectMessage {
            fn from(value: messages::InternalSubjectMessage) -> Self {
                Self {
                    message: value.message.into(),
                    subject: value.subject,
                    broker: value.broker,
                }
            }
        }

        impl From<Result<Vec<spin_message_types::SubjectMessage>,spin_message_types::MessageError>> for messages::Outcome {
            fn from(value: Result<Vec<spin_message_types::SubjectMessage>,spin_message_types::MessageError>) -> Self {
                match value {
                    Ok(vec) => messages::Outcome::Publish(vec.into_iter().map(|v| v.into()).collect()),
                    Err(err) => messages::Outcome::Error(err.to_string())
                }
            }
        }

        impl messages::Messages for Messages {
            fn handle_message(message: messages::InternalSubjectMessage) -> messages::Outcome {
                let message : spin_message_types::SubjectMessage = message.into();
                #func

                #func_name(message).into()
            }
        }

    )
    .into()
}
