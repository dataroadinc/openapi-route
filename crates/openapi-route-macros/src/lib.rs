//! Procedural macros for the openapi-route metadata crate.

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod handler;

/// Annotate a handler and generate an explicit route metadata constant.
#[proc_macro_attribute]
pub fn openapi_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let function = parse_macro_input!(input as syn::ItemFn);
    match handler::expand(args.into(), function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
