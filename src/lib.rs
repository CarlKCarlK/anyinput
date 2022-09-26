// todo rename to input-like-derive (or derive-input-like)
// todo remove cargo stuff features of syn no longer needed.
// todo use AST spans test so that problems with the user's syntax are reported correctly
// todo add nice error enum

// cmk Look more at https://github.com/dtolnay/syn/tree/master/examples/trace-var

use quote::quote;
use syn::__private::TokenStream; // todo don't use private
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Type::Path;
use syn::{parse_macro_input, parse_quote, parse_str, Block, Ident};
use syn::{FnArg, GenericParam, Generics, ItemFn, Pat, PatType, Signature, Stmt, Type};

// #[proc_macro_attribute]
pub fn input_like(_args: TokenStream, input: TokenStream) -> TokenStream {
    // panic!("input: {:#?}", &input);

    let old_item_fn = parse_macro_input!(input as ItemFn);
    // panic!("input: {:#?}", &input);

    // todo create unique names for macros identifiers with gensym
    let mut generic_gen = (0usize..).into_iter().map(|i| format!("S{i}"));
    let new_item_fn = transform_fn(old_item_fn, &mut generic_gen);

    TokenStream::from(quote!(#new_item_fn))
}

pub fn transform_fn(old_fn: ItemFn, generic_gen: &mut impl Iterator<Item = String>) -> ItemFn {
    // Check that function for special inputs such as 's: StringLike'. If found, replace with generics such as 's: S0' and remember.
    let (new_inputs, specials) = transform_inputs(&old_fn.sig.inputs, generic_gen);

    // For each special input found, define a new generic, for example, 'S0 : AsRef<str>'
    let new_generics = transform_generics(&old_fn.sig.generics.params, &specials);

    // For each special input found, add a statement defining a new local variable. For example, 'let s = s.as_ref();'
    let new_stmts = transform_stmts(&old_fn.block.stmts, &specials);

    // Create a new function with the transformed inputs, generics, and statements.
    // Use Rust's struct update syntax (https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/)
    // todo Is this the best way to create a new function from an old one?
    ItemFn {
        sig: Signature {
            generics: Generics {
                // todo: Define all constants outside the loop
                lt_token: parse_quote!(<),
                gt_token: parse_quote!(>),
                params: new_generics,
                ..old_fn.sig.generics.clone()
            },
            inputs: new_inputs,
            ..old_fn.sig.clone()
        },
        block: Box::new(Block {
            stmts: new_stmts,
            ..*old_fn.block
        }),
        ..old_fn
    }
}

struct Special {
    _name: Ident,
    ty: String,
}

fn first_and_only<T, I: Iterator<Item = T>>(mut iter: I) -> Option<T> {
    let first = iter.next()?;
    if iter.next().is_some() {
        None
    } else {
        Some(first)
    }
}

// Look for special inputs such as 's: StringLike'. If found, replace with generics like 's: S0'.
// Todo support: PathLike, IterLike<T>, ArrayLike<T> (including ArrayLike<PathLike>), NdArrayLike<T>, etc.
fn transform_inputs(
    old_inputs: &Punctuated<FnArg, Comma>,
    generic_gen: &mut impl Iterator<Item = String>,
) -> (Punctuated<FnArg, Comma>, Vec<Special>) {
    // For each old input, create a new input, transforming the type if it is special.
    let mut new_fn_args = Punctuated::<FnArg, Comma>::new();
    // Remember the names and types of the special inputs.
    let mut specials: Vec<Special> = vec![];

    // todo make this const somewhere
    let string_like_ident = syn::Ident::new("StringLike", proc_macro2::Span::call_site());

    for old_fn_arg in old_inputs {
        let mut found_special = false; // todo think of other ways to control the flow

        // If the input is 'Typed' (so not self), and
        // the 'pat' (aka variable) field is variant 'Ident' (so not, for example, a macro), and
        // the type is 'Path' (so not, for example, a macro), and
        // the one and only item in path is, for example, 'StringLike'
        // then replace the type with a generic type.
        //
        // see https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#destructuring-nested-structs-and-enums
        // todo: Do these struct contains Box to make them easier to modify?
        // The box pattern syntax is experimental and can't use used in stable Rust.

        if let FnArg::Typed(pat_type) = old_fn_arg {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                if let Path(type_path) = &*pat_type.ty {
                    if let Some(segment) = first_and_only(type_path.path.segments.iter()) {
                        if segment.ident == string_like_ident {
                            // Create a new input with a generic type and remember the name and type.
                            found_special = true;

                            let new_type_as_string =
                                generic_gen.next().expect("Can't gen a new generic name");
                            // todo use quote!
                            let new_ty = parse_str::<Type>(&new_type_as_string).unwrap();
                            let new_typed = FnArg::Typed(PatType {
                                ty: Box::new(new_ty),
                                ..pat_type.clone()
                            });
                            new_fn_args.push(new_typed);

                            let special = Special {
                                _name: pat_ident.ident.clone(),
                                ty: new_type_as_string,
                            };
                            specials.push(special);
                        }
                    }
                }
            }
        }
        if !found_special {
            new_fn_args.push(old_fn_arg.clone());
        }
    }
    (new_fn_args, specials)
}

// Define generics for each special input type. For example, 'S0 : AsRef<str>'
fn transform_generics(
    old_params: &Punctuated<GenericParam, Comma>,
    specials: &Vec<Special>,
) -> Punctuated<GenericParam, Comma> {
    let mut new_params = old_params.clone();
    for new_type in specials {
        let s = format!("{}: AsRef<str>", new_type.ty);
        new_params.push(parse_str(&s).expect("doesn't parse")); // todo use quote!
    }
    new_params
}

// For each special input type, define a new local variable. For example, 'let s = s.as_ref();'
// todo: Is there a way to use quote! to include the loop?
#[allow(clippy::ptr_arg)]
fn transform_stmts(old_stmts: &Vec<Stmt>, specials: &Vec<Special>) -> Vec<Stmt> {
    let mut new_stmts = old_stmts.clone();
    for (index, _special) in specials.iter().enumerate() {
        let name = &_special._name;
        let new_stmt = parse_quote! {
            let #name = #name.as_ref();
        };
        new_stmts.insert(index, new_stmt);
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

        let mut generic_gen = (0usize..).into_iter().map(|i| format!("S{i}"));
        let new_fn = transform_fn(old_fn, &mut generic_gen);
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
        let mut generic_gen = (0usize..).into_iter().map(|i| format!("S{i}"));
        let new_fn = transform_fn(old_fn, &mut generic_gen);
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
        let mut generic_gen = (0usize..).into_iter().map(|i| format!("S{i}"));
        let new_fn = transform_fn(old_fn, &mut generic_gen);
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

        let mut generic_gen = (0usize..).into_iter().map(|i| format!("S{i}"));
        let new_fn = transform_fn(old_fn, &mut generic_gen);
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
