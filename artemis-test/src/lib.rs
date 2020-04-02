pub mod queries;
pub use queries::*;

#[cfg(target_arch = "wasm32")]
pub mod client;

pub(crate) type Long = String;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::{prelude::*, JsValue};

    #[wasm_bindgen(typescript_custom_section)]
    const TS_APPEND_CONTENT: &'static str = r#"
    export type Long = string;
    "#;

    #[wasm_bindgen(start)]
    pub fn main() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();

        Ok(())
    }
}
