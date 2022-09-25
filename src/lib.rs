// cmk rename to input-like-derive (or derive-input-like)
// cmk remove cargo stuff features of syn no longer needed.
// cmk create unique names for macros with gensym
// cmk look a syn's macro workshop examples

// Parsing based on https://github.com/dtolnay/syn/tree/master/examples/trace-var
// #[cfg(test)]
// use proc_macro::TokenStream;
// #[cfg(test)]
// use quote::quote;
// #[cfg(test)]
// use syn::{parse_macro_input, ItemFn};

// struct Args {}

// impl Fold for Args {
//     fn fold_expr(&mut self, e: Expr) -> Expr {
//         match e {
//             // Expr::Assign(e) => {
//             //     // if self.should_print_expr(&e.left) {
//             //     //     self.assign_and_print(*e.left, &e.eq_token, *e.right)
//             //     // } else {
//             //     //     Expr::Assign(fold::fold_expr_assign(self, e))
//             //     // }
//             //     panic!("Assign not supported");
//             // }
//             // Expr::AssignOp(e) => {
//             //     // if self.should_print_expr(&e.left) {
//             //     //     self.assign_and_print(*e.left, &e.op, *e.right)
//             //     // } else {
//             //     //     Expr::AssignOp(fold::fold_expr_assign_op(self, e))
//             //     // }
//             //     panic!("AssignOp not supported");
//             // }
//             _ => fold::fold_expr(self, e),
//         }
//     }

//     fn fold_stmt(&mut self, s: Stmt) -> Stmt {
//         match s {
//             // Stmt::Local(s) => {
//             //     // if s.init.is_some() && self.should_print_pat(&s.pat) {
//             //     //     self.let_and_print(s)
//             //     // } else {
//             //     //     Stmt::Local(fold::fold_local(self, s))
//             //     // }
//             //     panic!("Local not supported");
//             // }
//             _ => fold::fold_stmt(self, s),
//         }
//     }
// }

// #[proc_macro_attribute]
// pub fn input_like(_: TokenStream, input: TokenStream) -> TokenStream {
//     // cmk 0 item
//     // panic!("item: {:#?}", &item);
//     // item

//     let input = parse_macro_input!(input as ItemFn);
//     // panic!("item: {:#?}", &input);

//     // let mut args = Args {};
//     // let output = args.fold_item_fn(input);

//     TokenStream::from(quote!(#input))
// }

// fn sample(s: &str) {
//     println!("{}", s.as_ref());
// }

#[cfg(test)]
mod tests {
    use prettyplease::unparse;
    use quote::quote;
    use syn::parse_str;
    use syn::{File, Generics, ItemFn, Signature};

    #[test]
    fn just_text() {
        let code = "fn main() { println!(); }"; // <S:AsRef<Str>>

        // using Rust's struct update syntax https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/
        let old_fn = parse_str::<ItemFn>(code).expect("doesn't parse");
        let mut new_params = old_fn.sig.generics.params.clone();
        new_params.push(parse_str("S : AsRef<str>").expect("doesn't parse"));
        let new_fn = ItemFn {
            sig: Signature {
                generics: Generics {
                    lt_token: syn::parse2(quote!(<)).unwrap(),
                    gt_token: syn::parse_str(">").unwrap(),
                    params: new_params,
                    ..old_fn.sig.generics.clone()
                },
                ..old_fn.sig.clone()
            },
            ..old_fn
        };

        // let &mut generics = &mut item_fn.sig.generics;
        // generics.lt_token = Some(syn::token::Lt::default());
        // generics.gt_token = Some(syn::token::Gt::default());

        println!("{:#?}", new_fn);

        let old_file = parse_str::<File>(code).expect("doesn't parse");
        let new_file = File {
            items: vec![syn::Item::Fn(new_fn)],
            ..old_file
        };
        let pp = unparse(&new_file);
        println!("{}", pp);
    }

    #[test]
    fn it_works() {
        // cmk update tests
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
