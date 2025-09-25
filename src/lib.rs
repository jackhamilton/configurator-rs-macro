extern crate proc_macro;
use proc_macro::TokenStream;
use crate::cli_builder::cli_builder_impl;

mod cli_builder;

#[proc_macro]
pub fn cli_builder(input: TokenStream) -> TokenStream {
    cli_builder_impl(input)
}
