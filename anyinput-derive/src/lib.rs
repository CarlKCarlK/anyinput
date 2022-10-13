#![doc = include_str!("../README.md")]

use anyinput_core::anyinput_core;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[proc_macro_error]
#[proc_macro_attribute]
pub fn anyinput(args: TokenStream, input: TokenStream) -> TokenStream {
    anyinput_core(args, input)
}
