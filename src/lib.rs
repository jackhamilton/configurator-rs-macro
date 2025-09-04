extern crate proc_macro;
use proc_macro::TokenStream;
use crate::freezable::freezable_impl;

mod freezable;

#[proc_macro_attribute]
pub fn freezable(args: TokenStream, input: TokenStream) -> TokenStream {
    freezable_impl(args, input)
}
