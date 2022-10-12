#![doc = include_str!("../README.md")]

use anyinput_core::{generic_gen_simple_factory, transform_fn};
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
#[proc_macro_error]
pub fn anyinput(_args: TokenStream, input: TokenStream) -> TokenStream {
    let old_item_fn = parse_macro_input!(input as ItemFn);
    let mut generic_gen = generic_gen_simple_factory();
    let new_item_fn = transform_fn(old_item_fn, &mut generic_gen);
    TokenStream::from(quote!(#new_item_fn))
}
