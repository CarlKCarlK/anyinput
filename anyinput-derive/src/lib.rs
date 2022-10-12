#![doc = include_str!("../README.md")]

use anyinput_core::anyinput_core;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn anyinput(_args: TokenStream, input: TokenStream) -> TokenStream {
    anyinput_core(_args, input)
}
