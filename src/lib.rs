// cmk rename to input-like-derive (or derive-input-like)

// Parsing based on https://github.com/dtolnay/syn/tree/master/examples/trace-var
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse_macro_input, Expr, ItemFn, Stmt,
};

struct Args {}

impl Fold for Args {
    fn fold_expr(&mut self, e: Expr) -> Expr {
        match e {
            // Expr::Assign(e) => {
            //     // if self.should_print_expr(&e.left) {
            //     //     self.assign_and_print(*e.left, &e.eq_token, *e.right)
            //     // } else {
            //     //     Expr::Assign(fold::fold_expr_assign(self, e))
            //     // }
            //     panic!("Assign not supported");
            // }
            // Expr::AssignOp(e) => {
            //     // if self.should_print_expr(&e.left) {
            //     //     self.assign_and_print(*e.left, &e.op, *e.right)
            //     // } else {
            //     //     Expr::AssignOp(fold::fold_expr_assign_op(self, e))
            //     // }
            //     panic!("AssignOp not supported");
            // }
            _ => fold::fold_expr(self, e),
        }
    }

    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        match s {
            // Stmt::Local(s) => {
            //     // if s.init.is_some() && self.should_print_pat(&s.pat) {
            //     //     self.let_and_print(s)
            //     // } else {
            //     //     Stmt::Local(fold::fold_local(self, s))
            //     // }
            //     panic!("Local not supported");
            // }
            _ => fold::fold_stmt(self, s),
        }
    }
}

#[proc_macro_attribute]
pub fn input_like(_: TokenStream, input: TokenStream) -> TokenStream {
    // cmk 0 item
    // panic!("item: {:#?}", &item);
    // item

    let input = parse_macro_input!(input as ItemFn);
    // panic!("item: {:#?}", &input);

    let mut args = Args {};
    let output = args.fold_item_fn(input);

    TokenStream::from(quote!(#output))

    // let ast: DeriveInput = syn::parse(input.clone()).unwrap();
    // // panic!("ast: {:#?}", &ast);
    // input
    // let ast: syn::DeriveInput = parse_macro_input!(item);
    // panic!("ast: {:#?}", ast);

    // // Construct a representation of Rust code as a syntax tree
    // // that we can manipulate
    // let ast: syn::DeriveInput = parse_macro_input!(item);
    // TokenStream::from(ast).into()

    // // panic!("My function name is: <{}>", ast.ident.to_string());
    // // TokenStream::new() //from(impl_input_like(&ast))

    // // Build the trait implementation
    // impl_input_like(&ast)
}

// fn impl_input_like(ast: &syn::DeriveInput) -> TokenStream {
//     ast.into_token_stream().into()
//     // let data = &ast.data;
//     // match data
//     // {
//     //     //syn::DataFn { attrs, vis, sig, block }
//     //     Fn() { attrs, vis, sig, block } => {
//     //         let name = &ast.ident;
//     //         let gen = quote! {
//     //             impl InputLike for #name {
//     //                 fn input_like(&self) -> Result<(), anyhow::Error> {
//     //                     // ...
//     //                 }
//     //             }
//     //         };
//     //         gen.into()
//     //     },
//     //     syn::Data::Fn() => {
//     //         panic!("cmk"),

//     //     },
//     //             _ => panic!("Only named fields are supported"),
//     //}
//     // let func = data
//     // TokenStream::from(&ast)
//     // let name = &ast.ident;
//     // let gen = quote! {
//     //     impl HelloMacro for #name {
//     //         fn hello_macro() {
//     //             println!("Hello, Macro! My name is {}", stringify!(#name));
//     //         }
//     //     }
//     // };
//     // gen.into()
// }

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // cmk update tests
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
