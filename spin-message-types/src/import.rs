// #![allow(clippy::all)]
// #![allow(unused_macros)]

// mod messages {
//     struct Exported;

//     wit_bindgen_rust::export!("exported.wit");

//     type Message = exported::Message;
//     type SubjectMessage = exported::SubjectMessage;
//     type Outcome = exported::Outcome;
//     type Metadata = exported::Metadata;

//     impl<T> exported::Exported for Exported<T> {
//         fn handle_message(message:exported::SubjectMessage,) -> exported::Outcome {
//             todo!()
//         }
//     }
// }

// pub use messages::*;

pub use message_macro::*;