extern crate proc_macro;
use proc_macro::TokenStream;

mod config_builder;

#[proc_macro]
pub fn config_builder(input: TokenStream) -> TokenStream {
    config_builder_impl(input)
}
