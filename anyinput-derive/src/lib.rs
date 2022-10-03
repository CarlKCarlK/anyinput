use anyinput_helper::{generic_gen_simple_factory, transform_fn};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn input_special(_args: TokenStream, input: TokenStream) -> TokenStream {
    // panic!("input: {:#?}", &input);

    let old_item_fn = parse_macro_input!(input as ItemFn);
    // panic!("input: {:#?}", &input);

    let mut generic_gen = generic_gen_simple_factory();
    let new_item_fn = transform_fn(old_item_fn, &mut generic_gen);

    TokenStream::from(quote!(#new_item_fn))
}
