// todo rename to input-like-derive (or derive-input-like)
// todo remove cargo stuff features of syn no longer needed.
// todo create unique names for macros identifiers with gensym
// todo use AST spans test so that problems with the user's syntax are reported correctly

// cmk Look more at https://github.com/dtolnay/syn/tree/master/examples/trace-var

use quote::quote;
use syn::__private::TokenStream;
use syn::parse_macro_input;
use syn::parse_str;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Type::Path;
use syn::{FnArg, Generics, ItemFn, Signature};

// #[proc_macro_attribute]
pub fn input_like(_args: TokenStream, input: TokenStream) -> TokenStream {
    // cmk 0 item
    // panic!("item: {:#?}", &item);
    // item

    let input = parse_macro_input!(input as ItemFn);
    // panic!("item: {:#?}", &input);

    // let mut args = Args {};
    // let output = args.fold_item_fn(input);

    TokenStream::from(quote!(#input))
}

pub fn transform_fn(old_fn: ItemFn) -> ItemFn {
    let new_fn_args = transform_inputs(&old_fn.sig.inputs);
    let new_params = transform_params(&old_fn.sig.generics.params);
    let new_stmts = transform_stmts(&old_fn.block.stmts);

    ItemFn {
        sig: Signature {
            generics: Generics {
                lt_token: syn::parse2(quote!(<)).unwrap(),
                gt_token: syn::parse_str(">").unwrap(), // todo use quote!
                params: new_params,
                ..old_fn.sig.generics.clone()
            },
            inputs: new_fn_args,
            ..old_fn.sig.clone()
        },
        block: Box::new(syn::Block {
            stmts: new_stmts,
            ..*old_fn.block
        }),
        ..old_fn
    }
}

fn transform_stmts(old_stmts: &Vec<syn::Stmt>) -> Vec<syn::Stmt> {
    let mut new_stmts = old_stmts.clone();
    new_stmts.insert(
        0,
        parse_str::<syn::Stmt>("let s = s.as_ref();").expect("doesn't parse"),
    );
    new_stmts
}

fn transform_params(
    old_params: &Punctuated<syn::GenericParam, Comma>,
) -> Punctuated<syn::GenericParam, Comma> {
    let mut new_params = old_params.clone();
    new_params.push(parse_str("S : AsRef<str>").expect("doesn't parse")); // todo use quote!
    new_params
}

fn transform_inputs(old_inputs: &Punctuated<FnArg, Comma>) -> Punctuated<FnArg, Comma> {
    let mut new_fn_args = Punctuated::<FnArg, Comma>::new();
    for old_fn_arg in old_inputs {
        let mut replaced = false; // todo think of other ways to control the flow
        if let FnArg::Typed(typed) = old_fn_arg {
            let old_ty = &*typed.ty;
            if let Path(type_path) = old_ty {
                let segments = &type_path.path.segments;
                // cmk what's up with multiple segments? why more than one?
                for segment in segments {
                    let ident = &segment.ident;
                    if ident == "StringLike" {
                        let new_ty = parse_str::<syn::Type>("S").expect("doesn't parse cmk");
                        // using Rust's struct update syntax https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/
                        let new_typed = FnArg::Typed(syn::PatType {
                            ty: Box::new(new_ty),
                            ..typed.clone()
                        });
                        new_fn_args.push(new_typed);
                        replaced = true;
                        break;
                    }
                }
            }
        }
        if !replaced {
            new_fn_args.push(old_fn_arg.clone());
        }
    }
    new_fn_args
}

#[cfg(test)]
mod tests {
    use prettyplease::unparse;
    use quote::quote;
    use syn::parse_macro_input;
    use syn::parse_str;

    use syn::{File, ItemFn};

    use crate::transform_fn;

    fn item_fn_to_string(item_fn: ItemFn) -> String {
        let old_file = parse_str::<File>("").expect("doesn't parse"); // todo is there a File::new?
        let new_file = File {
            items: vec![syn::Item::Fn(item_fn)],
            ..old_file
        };
        unparse(&new_file)
    }

    #[test]
    fn just_text() {
        let code = r#"pub fn any_str_len2(s: StringLike) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }"#;

        let old_fn = parse_str::<ItemFn>(code).expect("doesn't parse");
        let new_fn = transform_fn(old_fn);
        // println!("{:#?}", new_fn);
        let new_code = item_fn_to_string(new_fn);
        println!("{}", new_code);

        let expected_code_tokens = quote! {pub fn any_str_len2<S: AsRef<str>>(s: S) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len();
            Ok(len)
        }};

        let expected_item_fn = syn::parse2::<ItemFn>(expected_code_tokens).expect("doesn't parse");
        let expected_code = item_fn_to_string(expected_item_fn);
        assert_eq!(new_code, expected_code);
    }
}
