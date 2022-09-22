use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn input_like(_: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // cmk update tests
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
