// cmk rename to input-like-derive (or derive-input-like)
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{self, parse_macro_input};

#[proc_macro_attribute]
pub fn input_like(_: TokenStream, item: TokenStream) -> TokenStream {
    item
    // // Construct a representation of Rust code as a syntax tree
    // // that we can manipulate
    // let ast = parse_macro_input!(item);

    // // panic!("My function name is: <{}>", ast.ident.to_string());
    // // TokenStream::new() //from(impl_input_like(&ast))

    // // Build the trait implementation
    // impl_input_like(&ast)
}

fn impl_input_like(ast: &syn::DeriveInput) -> TokenStream {
    ast.into_token_stream().into()
    // let data = &ast.data;
    // match data
    // {
    //     //syn::DataFn { attrs, vis, sig, block }
    //     Fn() { attrs, vis, sig, block } => {
    //         let name = &ast.ident;
    //         let gen = quote! {
    //             impl InputLike for #name {
    //                 fn input_like(&self) -> Result<(), anyhow::Error> {
    //                     // ...
    //                 }
    //             }
    //         };
    //         gen.into()
    //     },
    //     syn::Data::Fn() => {
    //         panic!("cmk"),

    //     },
    //             _ => panic!("Only named fields are supported"),
    //}
    // let func = data
    // TokenStream::from(&ast)
    // let name = &ast.ident;
    // let gen = quote! {
    //     impl HelloMacro for #name {
    //         fn hello_macro() {
    //             println!("Hello, Macro! My name is {}", stringify!(#name));
    //         }
    //     }
    // };
    // gen.into()
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
