// ./crates/example-macro/src/lib.rs
extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn plugin_helper(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    tokens
}