mod queries;
pub use queries::*;

#[cfg(target_arch = "wasm32")]
pub mod client;

pub(crate) type Long = String;
