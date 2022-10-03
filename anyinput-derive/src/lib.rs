// cmk is the top-level project even needed?
// cmk later: run on a whole project, not just a single function
// cmk how does the documentation look in projects that use this macro?
// cmk why AnyIter but not AnyStr?
use anyinput_helper::{generic_gen_simple_factory, transform_fn};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn anyinput(_args: TokenStream, input: TokenStream) -> TokenStream {
    // panic!("input: {:#?}", &input);

    let old_item_fn = parse_macro_input!(input as ItemFn);
    // panic!("input: {:#?}", &input);

    let mut generic_gen = generic_gen_simple_factory();
    let new_item_fn = transform_fn(old_item_fn, &mut generic_gen);

    TokenStream::from(quote!(#new_item_fn))
}
