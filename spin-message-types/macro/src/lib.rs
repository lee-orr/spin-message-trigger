use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn message_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    const MESSAGE_COMPONENT_WIT: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../exported.wit"));

    let func = syn::parse_macro_input!(item as syn::ItemFn);

    quote!(
        wit_bindgen_rust::export!({src["messages"]: #MESSAGE_COMPONENT_WIT});

        struct Messages;

        pub type Message = messages::Message;
        pub type SubjectMessage = messages::SubjectMessage;
        pub type Outcome = messages::Outcome;
        pub type Metadata = messages::Metadata;

        impl messages::Messages for Messages {
                #func
        }

    )
    .into()
}
