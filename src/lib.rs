// todo rename to input-like-derive (or derive-input-like)
// todo remove cargo stuff features of syn no longer needed.
// todo create unique names for macros identifiers with gensym
// todo use AST spans test so that problems with the user's syntax are reported correctly

// cmk Look more at https://github.com/dtolnay/syn/tree/master/examples/trace-var

use quote::quote;
use syn::__private::TokenStream; // todo don't use private
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Type::Path;
use syn::{parse2, parse_macro_input, parse_str, Block};
use syn::{FnArg, GenericParam, Generics, ItemFn, Pat, PatType, Signature, Stmt, Type};

// #[proc_macro_attribute]
pub fn input_like(_args: TokenStream, input: TokenStream) -> TokenStream {
    // panic!("input: {:#?}", &input);

    let old_item_fn = parse_macro_input!(input as ItemFn);
    // panic!("input: {:#?}", &input);

    let new_item_fn = transform_fn(old_item_fn);

    TokenStream::from(quote!(#new_item_fn))
}

pub fn transform_fn(old_fn: ItemFn) -> ItemFn {
    // Look for function inputs such as 's: StringLike'. If found, replace with generics like 's: S0'.
    let (new_fn_args, new_likes) = transform_inputs(&old_fn.sig.inputs);

    // Define generics for each special input type. For example, 'S0 : AsRef<str>'
    let new_params = transform_params(&old_fn.sig.generics.params, &new_likes);

    // For each special input type, define a new local variable. For example, 'let s = s.as_ref();'
    let new_stmts = transform_stmts(&old_fn.block.stmts, &new_likes);

    // Create a new function with the transformed inputs, params, and statements.
    // using Rust's struct update syntax https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/
    // todo Is this the best way to create a new function from an old one?
    ItemFn {
        sig: Signature {
            generics: Generics {
                // todo is it bad to turn on the <> when there are no *Like inputs?
                lt_token: parse2(quote!(<)).unwrap(),
                gt_token: parse_str(">").unwrap(), // todo use quote!
                params: new_params,
                ..old_fn.sig.generics.clone()
            },
            inputs: new_fn_args,
            ..old_fn.sig.clone()
        },
        block: Box::new(Block {
            stmts: new_stmts,
            ..*old_fn.block
        }),
        ..old_fn
    }
}

struct Like {
    name: String,
    ty: String,
}

// Look for function inputs such as 's: StringLike'. If found, replace with generics like 's: S0'.
// Todo support: PathLike, ArrayLike<T> (including ArrayLike<PathLike>), NdArrayLike<T>, etc.
fn transform_inputs(
    old_inputs: &Punctuated<FnArg, Comma>,
) -> (Punctuated<FnArg, Comma>, Vec<Like>) {
    // For each old input, create a new input, transforming the type if it is a special type.
    let mut new_fn_args = Punctuated::<FnArg, Comma>::new();
    // Remember the names and types of the special inputs.
    let mut new_likes: Vec<Like> = vec![];

    for old_fn_arg in old_inputs {
        let mut found_special = false; // todo think of other ways to control the flow

        // If the input is 'Typed' (so not self), and
        // the 'variable' is variant 'Ident' (so not, for example, a macro), and
        // the type is 'Path' (so not, for example, a macro), and
        // the type's length is 1, and the type's one name is, for example, 'StringLike'
        if let FnArg::Typed(typed) = old_fn_arg {
            if let Pat::Ident(pat_ident) = &*typed.pat {
                if let Path(type_path) = &*typed.ty {
                    let segments = &type_path.path.segments;
                    if segments.len() == 1 {
                        let type_ident = &segments[0].ident;
                        if type_ident == "StringLike" {
                            // Create a new input with a generic type and remember the name and type.
                            found_special = true;

                            // todo use gensym to create unique names
                            let new_type_as_string = format!("S{}", new_likes.len());
                            // todo use quote!
                            let new_ty = parse_str::<Type>(&new_type_as_string).unwrap();
                            let new_typed = FnArg::Typed(PatType {
                                ty: Box::new(new_ty),
                                ..typed.clone()
                            });
                            new_fn_args.push(new_typed);

                            let new_like = Like {
                                name: pat_ident.ident.to_string(),
                                ty: new_type_as_string,
                            };
                            new_likes.push(new_like);
                        }
                    }
                }
            }
        }
        if !found_special {
            new_fn_args.push(old_fn_arg.clone());
        }
    }
    (new_fn_args, new_likes)
}

// Define generics for each special input type. For example, 'S0 : AsRef<str>'
fn transform_params(
    old_params: &Punctuated<GenericParam, Comma>,
    new_likes: &Vec<Like>,
) -> Punctuated<GenericParam, Comma> {
    let mut new_params = old_params.clone();
    for new_type in new_likes {
        let s = format!("{}: AsRef<str>", new_type.ty);
        new_params.push(parse_str(&s).expect("doesn't parse")); // todo use quote!
    }
    new_params
}

// For each special input type, define a new local variable. For example, 'let s = s.as_ref();'
#[allow(clippy::ptr_arg)]
fn transform_stmts(old_stmts: &Vec<Stmt>, new_likes: &Vec<Like>) -> Vec<Stmt> {
    let mut new_stmts = old_stmts.clone();
    for (index, new_like) in new_likes.iter().enumerate() {
        let s = format!("let {0} = {0}.as_ref();", new_like.name);
        new_stmts.insert(index, parse_str::<Stmt>(&s).expect("doesn't parse"));
    }
    new_stmts
}

#[cfg(test)]
mod tests {
    use prettyplease::unparse;
    use quote::quote;
    use syn::{parse2, parse_str};
    use syn::{File, Item, ItemFn};

    use crate::transform_fn;

    fn item_fn_to_string(item_fn: ItemFn) -> String {
        let old_file = parse_str::<File>("").expect("doesn't parse"); // todo is there a File::new?
        let new_file = File {
            items: vec![Item::Fn(item_fn)],
            ..old_file
        };
        unparse(&new_file)
    }

    #[test]
    fn one_input() {
        let code = r#"pub fn any_str_len1(s: StringLike) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }"#;
        let old_fn = parse_str::<ItemFn>(code).expect("doesn't parse");

        let new_fn = transform_fn(old_fn);
        // println!("{:#?}", new_fn);
        let new_code = item_fn_to_string(new_fn);
        println!("{}", new_code);

        let expected_code_tokens = quote! {pub fn any_str_len1<S0: AsRef<str>>(s: S0) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len();
            Ok(len)
        }};

        let expected_item_fn = parse2::<ItemFn>(expected_code_tokens).expect("doesn't parse");
        let expected_code = item_fn_to_string(expected_item_fn);
        assert_eq!(new_code, expected_code);
    }

    #[test]
    fn two_inputs() {
        let code = r#"pub fn any_str_len2(a: StringLike, b: StringLike) -> Result<usize, anyhow::Error> {
            let len = a.len() + b.len();
            Ok(len)
        }"#;
        let old_fn = parse_str::<ItemFn>(code).expect("doesn't parse");
        let new_fn = transform_fn(old_fn);
        let new_code = item_fn_to_string(new_fn);
        println!("{}", new_code);

        let expected_code_tokens = quote! {pub fn any_str_len2<S0: AsRef<str>, S1: AsRef<str>>(a: S0, b: S1) -> Result<usize, anyhow::Error> {
            let a = a.as_ref();
            let b = b.as_ref();
            let len = a.len() + b.len();
            Ok(len)
        }};

        let expected_item_fn = parse2::<ItemFn>(expected_code_tokens).expect("doesn't parse");
        let expected_code = item_fn_to_string(expected_item_fn);
        assert_eq!(new_code, expected_code);
    }

    #[test]
    fn zero_inputs() {
        let code = r#"pub fn any_str_len0() -> Result<usize, anyhow::Error> {
            let len = 0;
            Ok(len)
        }"#;
        let old_fn = parse_str::<ItemFn>(code).expect("doesn't parse");
        let new_fn = transform_fn(old_fn);
        let new_code = item_fn_to_string(new_fn);
        println!("{}", new_code);

        let expected_code_tokens = quote! {pub fn any_str_len0<>() -> Result<usize, anyhow::Error> {
            let len = 0;
            Ok(len)
        }};

        let expected_item_fn = parse2::<ItemFn>(expected_code_tokens).expect("doesn't parse");
        let expected_code = item_fn_to_string(expected_item_fn);
        assert_eq!(new_code, expected_code);
    }

    #[test]
    fn one_plus_two_input() {
        let code = r#"pub fn any_str_len1plus2(a: usize, s: StringLike, b: usize) -> Result<usize, anyhow::Error> {
            let len = s.len()+a+b;
            Ok(len)
        }"#;
        let old_fn = parse_str::<ItemFn>(code).expect("doesn't parse");

        let new_fn = transform_fn(old_fn);
        // println!("{:#?}", new_fn);
        let new_code = item_fn_to_string(new_fn);
        println!("{}", new_code);

        let expected_code_tokens = quote! {pub fn any_str_len1plus2<S0: AsRef<str>>(a: usize, s: S0, b: usize) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len()+a+b;
            Ok(len)
        }};

        let expected_item_fn = parse2::<ItemFn>(expected_code_tokens).expect("doesn't parse");
        let expected_code = item_fn_to_string(expected_item_fn);
        assert_eq!(new_code, expected_code);
    }
}
