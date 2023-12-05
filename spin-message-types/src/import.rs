pub use message_macro::*;
pub use wit_bindgen_rust::*;


mod platform {
wit_bindgen::generate!({
    path: "wit-spin",
    world: "platform"
});
}

pub use platform::fermyon::spin2_0_0 as spin;
pub use platform::wasi;
