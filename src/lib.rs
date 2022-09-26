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
use syn::{FnArg, GenericParam, Generics, ItemFn, Pat, PatType, Signature, Stmt};
use uuid::Uuid;

// #[proc_macro_attribute]
pub fn input_like(_args: TokenStream, input: TokenStream) -> TokenStream {
    // panic!("input: {:#?}", &input);

    let old_item_fn = parse_macro_input!(input as ItemFn);
    // panic!("input: {:#?}", &input);

    let new_item_fn = transform_fn(old_item_fn, &mut UuidGenerator::new());

    TokenStream::from(quote!(#new_item_fn))
}

pub fn transform_fn(old_fn: ItemFn, generic_gen: &mut impl Iterator<Item = syn::Type>) -> ItemFn {
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

struct UuidGenerator {
    counter: usize,
    uuid: String,
}

impl UuidGenerator {
    fn new() -> Self {
        Self {
            uuid: Uuid::new_v4().to_string().replace('-', "_"),
            counter: 0,
        }
    }
}

impl Iterator for UuidGenerator {
    type Item = syn::Type;

    fn next(&mut self) -> Option<Self::Item> {
        let s = format!("U{}_{}", self.uuid, self.counter);
        let result = parse_str(&s).expect("parse failure"); // cmk
        self.counter += 1;
        Some(result)
    }
}

struct Special {
    _name: Ident,
    ty: syn::Type,
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
    generic_gen: &mut impl Iterator<Item = syn::Type>,
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

                            let next = generic_gen.next(); // cmk
                            let new_type_ident = next.expect("Can't gen a new generic name");
                            let new_type = parse_quote!(#new_type_ident);
                            let new_typed = FnArg::Typed(PatType {
                                ty: Box::new(new_type),
                                ..pat_type.clone()
                            });
                            new_fn_args.push(new_typed);

                            let special = Special {
                                _name: pat_ident.ident.clone(),
                                ty: new_type_ident,
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
#[allow(clippy::ptr_arg)]
fn transform_generics(
    old_params: &Punctuated<GenericParam, Comma>,
    specials: &Vec<Special>,
) -> Punctuated<GenericParam, Comma> {
    let mut new_params = old_params.clone();
    for new_type in specials.iter().map(|s| &s.ty) {
        new_params.push(parse_quote!(#new_type : AsRef<str>));
    }
    new_params
}

// For each special input type, define a new local variable. For example, 'let s = s.as_ref();'
#[allow(clippy::ptr_arg)]
fn transform_stmts(old_stmts: &Vec<Stmt>, specials: &Vec<Special>) -> Vec<Stmt> {
    let mut new_stmts = old_stmts.clone();
    for (index, name) in specials.iter().map(|special| &special._name).enumerate() {
        let new_stmt = parse_quote! {
            let #name = #name.as_ref();
        };
        new_stmts.insert(index, new_stmt);
    }
    new_stmts
}

#[cfg(test)]
mod tests {
    // cmk use prettyplease::unparse;
    use crate::{transform_fn, UuidGenerator};
    use syn::{parse_quote, parse_str};

    fn str_to_type(s: &str) -> syn::Type {
        parse_str(s).unwrap()
    }

    // cmk
    // fn item_fn_to_string(item_fn: ItemFn) -> String {
    //     let old_file = parse_str::<File>("").expect("doesn't parse"); // todo is there a File::new?
    //     let new_file = File {
    //         items: vec![Item::Fn(item_fn)],
    //         ..old_file
    //     };
    //     unparse(&new_file)
    // }

    fn generic_gen_test_factory() -> impl Iterator<Item = syn::Type> + 'static {
        (0usize..)
            .into_iter()
            .map(|i| str_to_type(&format!("S{i}")))
    }

    #[test]
    fn uuid() {
        let mut uuid_generator = UuidGenerator::new();
        for i in 0..10 {
            let _ = uuid_generator.next();
            println!("{:#?}", i);
        }
    }

    #[test]
    fn one_input() {
        let before = parse_quote! {
        pub fn any_str_len1(s: StringLike) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }        };
        let expected = parse_quote! {
        pub fn any_str_len1<S0: AsRef<str>>(s: S0) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len();
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_eq!(after, expected);
    }

    #[test]
    fn two_inputs() {
        let before = parse_quote! {
        pub fn any_str_len2(a: StringLike, b: StringLike) -> Result<usize, anyhow::Error> {
            let len = a.len() + b.len();
            Ok(len)
        }};
        let expected = parse_quote! {
        pub fn any_str_len2<S0: AsRef<str>, S1: AsRef<str>>(a: S0, b: S1) -> Result<usize, anyhow::Error> {
            let a = a.as_ref();
            let b = b.as_ref();
            let len = a.len() + b.len();
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_eq!(after, expected);
    }

    #[test]
    fn zero_inputs() {
        let before = parse_quote! {
        pub fn any_str_len0() -> Result<usize, anyhow::Error> {
            let len = 0;
            Ok(len)
        }};
        let expected = parse_quote! {
        pub fn any_str_len0<>() -> Result<usize, anyhow::Error> {
            let len = 0;
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_eq!(after, expected);
    }

    #[test]
    fn one_plus_two_input() {
        let before = parse_quote! {
        pub fn any_str_len1plus2(a: usize, s: StringLike, b: usize) -> Result<usize, anyhow::Error> {
            let len = s.len()+a+b;
            Ok(len)
        }};
        let expected = parse_quote! {
        pub fn any_str_len1plus2<S0: AsRef<str>>(a: usize, s: S0, b: usize) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len()+a+b;
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_eq!(after, expected);
    }

    #[test]
    fn one_input_uuid() {
        let before = parse_quote! {pub fn any_str_len1(s: StringLike) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }};
        let _ = transform_fn(before, &mut generic_gen_test_factory());
    }
}
