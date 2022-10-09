// cmk is the top-level project even needed?
// cmk later: run on a whole project, not just a single function
// cmk how does the documentation look in projects that use this macro?
// cmk why AnyIter but not AnyStr?
use anyinput_core::anyinput as anyinput_internal;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn anyinput(_args: TokenStream, input: TokenStream) -> TokenStream {
    anyinput_internal(_args, input)
}
