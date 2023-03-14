#![allow(clippy::all)]
#![allow(unused_macros)]

mod messages {
    use wit_bindgen::generate;

    generate!("messages");
}

pub use messages::*;
