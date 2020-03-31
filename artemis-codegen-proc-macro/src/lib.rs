use artemis_codegen::wasm::{wasm_client as generate, WasmClientInput};
use proc_macro::TokenStream;

#[proc_macro]
pub fn wasm_client(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as WasmClientInput);
    generate(input).into()
}
