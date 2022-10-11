#![doc = include_str!("../README.md")]
// todo use AST spans test so that problems with the user's syntax are reported correctly
//           see quote_spanned! in https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs

// todo could nested .as_ref(), .into_iter(), and .into() be replaced with a single method?
use std::str::FromStr;

use quote::quote;
use strum::EnumString;
use syn::__private::TokenStream;
use syn::fold::{fold_type_path, Fold};
// todo don't use private
use syn::{
    parse_macro_input, parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg,
    GenericArgument, GenericParam, Generics, ItemFn, Lifetime, Pat, PatIdent, PatType,
    PathArguments, PathSegment, Signature, Stmt, Type, TypePath,
};
pub fn generic_gen_simple_factory() -> impl Iterator<Item = String> + 'static {
    (0usize..).into_iter().map(|i| format!("{i}"))
}

pub fn anyinput(_args: TokenStream, input: TokenStream) -> TokenStream {
    let old_item_fn = parse_macro_input!(input as ItemFn);
    let mut generic_gen = generic_gen_simple_factory();
    let new_item_fn = transform_fn(old_item_fn, &mut generic_gen);
    TokenStream::from(quote!(#new_item_fn))
}

#[derive(Debug, Clone, EnumString)]
#[allow(clippy::enum_variant_names)]
enum Special {
    AnyArray,
    AnyString,
    AnyPath,
    AnyIter,
    AnyNdArray,
}

// todo do something interesting with 2d ndarray/views

impl Special {
    fn should_add_lifetime(&self) -> bool {
        match self {
            Special::AnyArray | Special::AnyString | Special::AnyPath | Special::AnyIter => false,
            Special::AnyNdArray => true,
        }
    }
    fn special_to_generic_param(
        &self,
        new_type: &TypePath,
        sub_type: Option<&Type>,
        lifetime: Option<Lifetime>,
    ) -> GenericParam {
        match &self {
            Special::AnyString => {
                if sub_type.is_some() {
                    panic!("AnyString should not have a generic parameter, so AnyString, not AnyString<{}>.", quote!(#sub_type));
                };
                assert!(lifetime.is_none(), "AnyString should not have a lifetime.");
                parse_quote!(#new_type : AsRef<str>)
            }
            Special::AnyPath => {
                if sub_type.is_some() {
                    panic!(
                        "AnyPath should not have a generic parameter, so AnyPath, not AnyPath<{}>.",
                        quote!(#sub_type)
                    );
                };
                assert!(lifetime.is_none(), "AnyPath should not have a lifetime.");
                parse_quote!(#new_type : AsRef<std::path::Path>)
            }
            Special::AnyArray => {
                let sub_type = sub_type.expect("AnyArray expects a generic parameter, for example, AnyArray<usize> or AnyArray<AnyString>.");
                assert!(lifetime.is_none(), "AnyArray should not have a lifetime.");
                parse_quote!(#new_type : AsRef<[#sub_type]>)
            }
            Special::AnyIter => {
                let sub_type = sub_type.expect(
                    "AnyIter expects a generic parameter, for example, AnyIter<usize> or AnyIter<AnyString>.",
                );
                assert!(lifetime.is_none(), "AnyIter should not have a lifetime.");
                parse_quote!(#new_type : IntoIterator<Item = #sub_type>)
            }
            Special::AnyNdArray => {
                let sub_type = sub_type.expect("AnyNdArray expects a generic parameter, for example, AnyNdArray<usize> or AnyNdArray<AnyString>.");
                let lifetime =
                    lifetime.expect("Internal error: AnyNdArray should be given a lifetime.");
                parse_quote!(#new_type: Into<ndarray::ArrayView1<#lifetime, #sub_type>>)
            }
        }
    }

    fn pat_ident_to_stmt(&self, pat_ident: &PatIdent) -> Stmt {
        let name = &pat_ident.ident;
        match &self {
            Special::AnyArray | Special::AnyString | Special::AnyPath => {
                parse_quote! {
                    let #name = #name.as_ref();
                }
            }
            Special::AnyIter => {
                parse_quote! {
                    let #name = #name.into_iter();
                }
            }
            Special::AnyNdArray => {
                parse_quote! {
                    let #name = #name.into();
                }
            }
        }
    }
}

pub fn transform_fn(old_fn: ItemFn, generic_gen: &mut impl Iterator<Item = String>) -> ItemFn {
    // Start the functions current generic definitions and statements
    let init = DeltaFnArgs {
        fn_args: Punctuated::<FnArg, Comma>::new(),
        generic_params: old_fn.sig.generics.params.clone(),
        stmts: old_fn.block.stmts,
    };

    // Transform each old argument of the function, accumulating the new arguments, new generic definitions and new statements
    let delta_fun_args = (old_fn.sig.inputs)
        .iter()
        .map(|old_fn_arg| transform_fn_arg(old_fn_arg, generic_gen))
        .fold(init, |mut delta_fun_args, delta_fun_arg| {
            delta_fun_args.merge(delta_fun_arg);
            delta_fun_args
        });

    // Create a new function with the transformed inputs and accumulated generic definitions, and statements.
    // Use Rust's struct update syntax (https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/)
    // todo Is this the best way to create a new function from an old one?
    ItemFn {
        sig: Signature {
            generics: Generics {
                // todo: Define all constants outside the loop
                lt_token: parse_quote!(<),
                gt_token: parse_quote!(>),
                params: delta_fun_args.generic_params,
                ..old_fn.sig.generics.clone()
            },
            inputs: delta_fun_args.fn_args,
            ..old_fn.sig.clone()
        },
        block: Box::new(Block {
            stmts: delta_fun_args.stmts,
            ..*old_fn.block
        }),
        ..old_fn
    }
}

// pub struct UuidGenerator {
//     counter: usize,
//     uuid: String,
// }

// impl Default for UuidGenerator {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// impl UuidGenerator {
//     pub fn new() -> Self {
//         Self {
//             uuid: Uuid::new_v4().to_string().replace('-', ""),
//             counter: 0,
//         }
//     }
// }

// impl Iterator for UuidGenerator {
//     type Item = String;

//     fn next(&mut self) -> Option<Self::Item> {
//         let s = format!("{}_{}", self.uuid, self.counter);
//         self.counter += 1;
//         Some(s)
//     }
// }

fn first_and_only<T, I: Iterator<Item = T>>(mut iter: I) -> Option<T> {
    let first = iter.next()?;
    if iter.next().is_some() {
        None
    } else {
        Some(first)
    }
}

// Look for special inputs such as 's: AnyString'. If found, replace with generics special 's: S0'.
// Todo support: AnyPath, AnyIter<T>, AnyArray<T> (including AnyArray<AnyPath>), NdArraySpecial<T>, etc.

// for each input, if it is top-level special, replace it with generic(s) and remember the generic(s) and the top-level variable.
// v: i32 -> v: i32, <>, {}
// v: AnyString -> v: S0, <S0: AsRef<str>>, {let v = v.as_ref();}
// v: AnyIter<i32> -> v: S0, <S0: IntoIterator<Item = i32>>, {let v = v.into_iter();}
// v: AnyIter<AnyString> -> v: S0, <S0: IntoIterator<Item = S1>, S1: AsRef<str>>, {let v = v.into_iter();}
// v: AnyIter<AnyIter<i32>> -> v: S0, <S0: IntoIterator<Item = S1>, S1: IntoIterator<Item = i32>>, {let v = v.into_iter();}
// v: AnyIter<AnyIter<AnyString>> -> v: S0, <S0: IntoIterator<Item = S1>, S1: IntoIterator<Item = S2>, S2: AsRef<str>>, {let v = v.into_iter();}
// v: [AnyString] -> v: [S0], <S0: AsRef<str>>, {}

struct DeltaFnArgs {
    fn_args: Punctuated<FnArg, Comma>,
    generic_params: Punctuated<GenericParam, Comma>,
    stmts: Vec<Stmt>,
}

impl DeltaFnArgs {
    fn merge(&mut self, delta_fn_arg: DeltaFnArg) {
        self.fn_args.push(delta_fn_arg.fn_arg);
        self.generic_params.extend(delta_fn_arg.generic_params);
        for (index, stmt) in delta_fn_arg.stmts.into_iter().enumerate() {
            self.stmts.insert(index, stmt);
        }
    }
}

#[derive(Debug)]
// the new function input, any statements to add, and any new generic definitions.
struct DeltaFnArg {
    fn_arg: FnArg,
    generic_params: Vec<GenericParam>,
    stmts: Vec<Stmt>,
}

fn transform_fn_arg(
    old_fn_arg: &FnArg,
    generic_gen: &mut impl Iterator<Item = String>,
) -> DeltaFnArg {
    // If the function input is normal (not self, not a macro, etc) ...
    if let Some((pat_ident, pat_type)) = is_normal_fn_arg(old_fn_arg) {
        // Replace any specials in the type with generics.
        let (delta_pat_type, new_pat_type) = replace_any_specials(pat_type.clone(), generic_gen);

        // Return the new function input, any statements to add, and any new generic definitions.
        DeltaFnArg {
            fn_arg: FnArg::Typed(new_pat_type),
            stmts: delta_pat_type.generate_any_stmts(pat_ident),
            generic_params: delta_pat_type.generic_params,
        }
    } else {
        // if input is not normal, return it unchanged.
        DeltaFnArg {
            fn_arg: old_fn_arg.clone(),
            generic_params: vec![],
            stmts: vec![],
        }
    }
}

impl DeltaPatType<'_> {
    fn generate_any_stmts(&self, pat_ident: &PatIdent) -> Vec<Stmt> {
        if let Some(special) = &self.last_special {
            vec![special.pat_ident_to_stmt(pat_ident)]
        } else {
            vec![]
        }
    }
}

// A function argument is normal if it is not self, not a macro, etc.
fn is_normal_fn_arg(fn_arg: &FnArg) -> Option<(&PatIdent, &PatType)> {
    if let FnArg::Typed(pat_type) = fn_arg {
        if let Pat::Ident(pat_ident) = &*pat_type.pat {
            if let Type::Path(_) = &*pat_type.ty {
                return Some((pat_ident, pat_type));
            }
        }
    }
    None
}

#[allow(clippy::ptr_arg)]
fn replace_any_specials(
    old_pat_type: PatType,
    generic_gen: &mut impl Iterator<Item = String>,
) -> (DeltaPatType, PatType) {
    // Search type and its (sub)subtypes for specials starting at the deepest level.
    // When one is found, replace it with a generic.
    // Finally, return the new type and a list of the generic definitions.
    // Also, if the top-level type was special, return the special type.

    let mut delta_pat_type = DeltaPatType {
        generic_params: vec![],
        generic_gen,
        last_special: None,
    };
    let new_path_type = delta_pat_type.fold_pat_type(old_pat_type);

    (delta_pat_type, new_path_type)
}

struct DeltaPatType<'a> {
    generic_params: Vec<GenericParam>,
    generic_gen: &'a mut dyn Iterator<Item = String>,
    last_special: Option<Special>,
}

fn camel_case_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (index, c) in s.chars().enumerate() {
        if index > 0 && c.is_uppercase() {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

impl Fold for DeltaPatType<'_> {
    fn fold_type_path(&mut self, type_path: TypePath) -> TypePath {
        // println!("fold_type_path (before): {:?}", quote!(#type_path));

        // Search for any special (sub)subtypes, replacing them with generics.
        let mut type_path = fold_type_path(self, type_path);

        // If this top-level type is special, replace it with a generic.
        if let Some((segment, special)) = is_special_type_path(&type_path) {
            self.last_special = Some(special.clone()); // remember which kind of special found

            let suffix = self
                .generic_gen
                .next()
                .expect("Internal error: ran out of generic suffixes");
            let generic_name = format!("{:?}{}", &special, suffix); // todo implement display and remove "?"
            type_path =
                parse_str(&generic_name).expect("Internal error: failed to parse generic name");

            // Define the generic type, e.g. S23: AsRef<str>, and remember it.
            let sub_type = has_sub_type(segment.arguments); // Find anything inside angle brackets.

            let maybe_lifetime = if special.should_add_lifetime() {
                let suffix = &self
                    .generic_gen
                    .next()
                    .expect("Internal error: ran out of generic suffixes");
                let snake_case = camel_case_to_snake_case(&format!("{:?}", &special));
                let lifetime_name = format!("'{}{}", snake_case, suffix,);
                let lifetime: Lifetime = parse_str(&lifetime_name)
                    .expect("Internal error: failed to parse lifetime name");
                let generic_param: GenericParam = parse_quote! { #lifetime };
                self.generic_params.push(generic_param);

                Some(lifetime)
            } else {
                None
            };

            let generic_param =
                special.special_to_generic_param(&type_path, sub_type.as_ref(), maybe_lifetime);
            self.generic_params.push(generic_param);
        } else {
            self.last_special = None;
        }
        // println!("fold_type_path (after): {}", quote!(#type_path));
        type_path
    }
}

fn has_sub_type(args: PathArguments) -> Option<Type> {
    match args {
        PathArguments::None => None,
        PathArguments::AngleBracketed(ref args) => {
            let arg = first_and_only(args.args.iter()).unwrap_or_else(|| {
                panic!(
                    "Expected at most one generic parameter, not '{}'",
                    quote!(#args)
                )
            });
            // println!("arg: {}", quote!(#arg));
            if let GenericArgument::Type(sub_type2) = arg {
                Some(sub_type2.clone())
            } else {
                panic!(
                    "Expected generic parameter to be a type, not '{}'",
                    quote!(#args)
                )
            }
        }
        PathArguments::Parenthesized(_) => {
            panic!("Expected <..> generic parameter,  not '{}'", quote!(#args))
        }
    }
}

fn is_special_type_path(type_path: &TypePath) -> Option<(PathSegment, Special)> {
    // A special type path has exactly one segment and a name from the Special enum.
    if let Some(segment) = first_and_only(type_path.path.segments.iter()) {
        if let Ok(special) = Special::from_str(segment.ident.to_string().as_ref()) {
            Some((segment.clone(), special))
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{generic_gen_simple_factory, transform_fn, DeltaPatType};
    use quote::quote;
    use syn::{fold::Fold, parse_quote, ItemFn};
    #[cfg(feature = "ndarray")]
    use syn::{GenericParam, Lifetime};

    fn assert_item_fn_eq(after: &ItemFn, expected: &ItemFn) {
        if after == expected {
            return;
        }

        let after_str = format!("{}", quote!(#after));
        let expected_str = format!("{}", quote!(#expected));
        if after_str == expected_str {
            return;
        }
        println!(
            "{}",
            colored_diff::PrettyDifference {
                expected: &expected_str,
                actual: &after_str,
            }
        );
        println!("expected: {}", expected_str);
        println!("after   : {}", after_str);
        panic!("after != expected");
    }

    // #[test]
    // fn uuid() {
    //     let mut uuid_generator = UuidGenerator::new();
    //     for i in 0..10 {
    //         let _ = uuid_generator.next();
    //         println!("{:#?}", i);
    //     }
    // }

    #[test]
    fn one_input() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyString) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let expected = parse_quote! {
            pub fn any_str_len<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
                let s = s.as_ref();
                let len = s.len();
                Ok(len)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_str_len<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len();
            Ok(len)
        }
        assert!(any_str_len("abc").is_ok());
    }

    #[test]
    fn two_inputs() {
        let before = parse_quote! {
            pub fn any_str_len(a: AnyString, b: AnyString) -> Result<usize, anyhow::Error> {
                let len = a.len() + b.len();
                Ok(len)
            }
        };
        let expected = parse_quote! {
            pub fn any_str_len<AnyString0: AsRef<str>, AnyString1: AsRef<str>>(
                a: AnyString0,
                b: AnyString1
            ) -> Result<usize, anyhow::Error> {
                let b = b.as_ref();
                let a = a.as_ref();
                let len = a.len() + b.len();
                Ok(len)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_str_len<AnyString0: AsRef<str>, AnyString1: AsRef<str>>(
            a: AnyString0,
            b: AnyString1,
        ) -> Result<usize, anyhow::Error> {
            let b = b.as_ref();
            let a = a.as_ref();
            let len = a.len() + b.len();
            Ok(len)
        }

        let s = "1234".to_string();
        assert_eq!(any_str_len("abc", s).unwrap(), 7);
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

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);
    }

    #[test]
    fn one_plus_two_input() {
        let before = parse_quote! {
            pub fn any_str_len_plus2(a: usize, s: AnyString, b: usize) -> Result<usize, anyhow::Error> {
                let len = s.len()+a+b;
                Ok(len)
            }
        };
        let expected = parse_quote! {
            pub fn any_str_len_plus2<AnyString0: AsRef<str>>(
                a: usize,
                s: AnyString0,
                b: usize
            ) -> Result<usize, anyhow::Error> {
                let s = s.as_ref();
                let len = s.len() + a + b;
                Ok(len)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_str_len_plus2<AnyString0: AsRef<str>>(
            a: usize,
            s: AnyString0,
            b: usize,
        ) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len() + a + b;
            Ok(len)
        }
        assert_eq!(any_str_len_plus2(1, "abc", 2).unwrap(), 6);
    }

    #[test]
    fn one_input_uuid() {
        let before = parse_quote! {pub fn any_str_len(s: AnyString) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }};
        let _ = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    fn one_path_input() {
        let before = parse_quote! {
        pub fn any_count_path(p: AnyPath) -> Result<usize, anyhow::Error> {
            let count = p.iter().count();
            Ok(count)
        }
          };
        let expected = parse_quote! {
            pub fn any_count_path<AnyPath0: AsRef<std::path::Path>>(
                p: AnyPath0
            ) -> Result<usize, anyhow::Error> {
                let p = p.as_ref();
                let count = p.iter().count();
                Ok(count)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_path<AnyPath0: AsRef<std::path::Path>>(
            p: AnyPath0,
        ) -> Result<usize, anyhow::Error> {
            let p = p.as_ref();
            let count = p.iter().count();
            Ok(count)
        }
        assert_eq!(any_count_path("abc/ed").unwrap(), 2);
    }

    #[test]
    fn one_iter_usize_input() {
        let before = parse_quote! {
            pub fn any_count_iter(i: AnyIter<usize>) -> Result<usize, anyhow::Error> {
                let count = i.count();
                Ok(count)
            }
        };
        let expected = parse_quote! {
            pub fn any_count_iter<AnyIter0: IntoIterator<Item = usize>>(
                i: AnyIter0
            ) -> Result<usize, anyhow::Error> {
                let i = i.into_iter();
                let count = i.count();
                Ok(count)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<AnyIter0: IntoIterator<Item = usize>>(
            i: AnyIter0,
        ) -> Result<usize, anyhow::Error> {
            let i: <AnyIter0 as IntoIterator>::IntoIter = i.into_iter();
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    }

    #[test]
    fn one_iter_i32() {
        let before = parse_quote! {
        pub fn any_count_iter(i: AnyIter<i32>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
            };
        let expected = parse_quote! {
            pub fn any_count_iter<AnyIter0: IntoIterator<Item = i32>>(
                i: AnyIter0
            ) -> Result<usize, anyhow::Error> {
                let i = i.into_iter();
                let count = i.count();
                Ok(count)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<AnyIter0: IntoIterator<Item = i32>>(
            i: AnyIter0,
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    }

    #[test]
    fn one_iter_t() {
        let before = parse_quote! {
        pub fn any_count_iter<T>(i: AnyIter<T>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
           };
        let expected = parse_quote! {
            pub fn any_count_iter<T, AnyIter0: IntoIterator<Item = T>>(
                i: AnyIter0
            ) -> Result<usize, anyhow::Error> {
                let i = i.into_iter();
                let count = i.count();
                Ok(count)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<T, AnyIter0: IntoIterator<Item = T>>(
            i: AnyIter0,
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    }

    #[test]
    fn one_iter_path() {
        let before = parse_quote! {
        pub fn any_count_iter(i: AnyIter<AnyPath>) -> Result<usize, anyhow::Error> {
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
           };
        let expected = parse_quote! {
                pub fn any_count_iter<
                AnyPath0: AsRef<std::path::Path>,
                AnyIter1: IntoIterator<Item = AnyPath0>
            >(
                i: AnyIter1
            ) -> Result<usize, anyhow::Error> {
                let i = i.into_iter();
                let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
                Ok(sum_count)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<
            AnyPath0: AsRef<std::path::Path>,
            AnyIter1: IntoIterator<Item = AnyPath0>,
        >(
            i: AnyIter1,
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_iter(["a/b", "d"]).unwrap(), 3);
    }

    #[test]
    fn one_vec_path() {
        let before = parse_quote! {
            pub fn any_count_vec(
                i: Vec<AnyPath>,
            ) -> Result<usize, anyhow::Error> {
                let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
                Ok(sum_count)
            }
        };
        let expected = parse_quote! {
        pub fn any_count_vec<AnyPath0: AsRef<std::path::Path>>(
            i: Vec<AnyPath0>
        ) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }};

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_vec<AnyPath0: AsRef<std::path::Path>>(
            i: Vec<AnyPath0>,
        ) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
    }

    #[test]
    fn fold_one_path() {
        let before = parse_quote! {AnyIter<AnyPath> };
        println!("before: {}", quote!(before));
        let mut gen = generic_gen_simple_factory();
        let mut struct1 = DeltaPatType {
            generic_params: vec![],
            generic_gen: &mut gen,
            last_special: None,
        };
        let result = struct1.fold_type(before);
        for generic_param in struct1.generic_params {
            println!("generic_param: {}", quote!(#generic_param));
        }

        println!("result: {}", quote!(#result));
    }

    #[test]
    fn one_array_usize_input() {
        let before = parse_quote! {
        pub fn any_array_len(a: AnyArray<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }
          };
        let expected = parse_quote! {
            pub fn any_array_len<AnyArray0: AsRef<[usize]>>(
                a: AnyArray0
            ) -> Result<usize, anyhow::Error> {
                let a = a.as_ref();
                let len = a.len();
                Ok(len)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_array_len<AnyArray0: AsRef<[usize]>>(
            a: AnyArray0,
        ) -> Result<usize, anyhow::Error> {
            let a = a.as_ref();
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_array_len([1, 2, 3]).unwrap(), 3);
    }

    #[cfg(feature = "ndarray")]
    #[test]
    fn understand_lifetime_parse() {
        let a = Lifetime::new("'a", syn::__private::Span::call_site());
        println!("a: {}", quote!(#a));
        let b: Lifetime = parse_quote!('a);
        println!("b: {}", quote!(#b));

        let _generic_param: GenericParam = parse_quote!(S1: Into<ndarray::ArrayView1<'S0, S2>>);
        println!("gp: {}", quote!(#_generic_param));
        println!("done");
    }

    #[cfg(feature = "ndarray")]
    #[test]
    fn one_ndarray_usize_input() {
        let before = parse_quote! {
        pub fn any_array_len(a: AnyNdArray<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }        };
        let expected = parse_quote! {
                pub fn any_array_len<
                'any_nd_array1,
                AnyNdArray0: Into<ndarray::ArrayView1<'any_nd_array1, usize>>
            >(
                a: AnyNdArray0
            ) -> Result<usize, anyhow::Error> {
                let a = a.into();
                let len = a.len();
                Ok(len)
            }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        // The lines are long enough that Clippy would like a comma after
        // a:AnyNdArray0, but the macro doesn't do that because
        // it doesn't know the line length.
        pub fn any_array_len<
            'any_nd_array1,
            AnyNdArray0: Into<ndarray::ArrayView1<'any_nd_array1, usize>>,
        >(
            a: AnyNdArray0,
        ) -> Result<usize, anyhow::Error> {
            let a = a.into();
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_array_len([1, 2, 3].as_ref()).unwrap(), 3);
    }

    #[test]
    #[cfg(feature = "ndarray")]
    fn complex() {
        let before = parse_quote! {
            pub fn complex_total(
                a: usize,
                b: AnyIter<Vec<AnyArray<AnyPath>>>,
                c: AnyNdArray<usize>
            ) -> Result<usize, anyhow::Error> {
                let mut total = a + c.sum();
                for vec in b {
                    for any_array in vec {
                        let any_array = any_array.as_ref();
                        for any_path in any_array.iter() {
                            let any_path = any_path.as_ref();
                            total += any_path.iter().count();
                        }
                    }
                }
                Ok(total)
                }
        };
        let expected = parse_quote! {
            pub fn complex_total<
            'any_nd_array4,
            AnyPath0: AsRef<std::path::Path>,
            AnyArray1: AsRef<[AnyPath0]>,
            AnyIter2: IntoIterator<Item = Vec<AnyArray1>>,
            AnyNdArray3: Into<ndarray::ArrayView1<'any_nd_array4, usize>>
        >(
            a: usize,
            b: AnyIter2,
            c: AnyNdArray3
        ) -> Result<usize, anyhow::Error> {
            let c = c.into();
            let b = b.into_iter();
            let mut total = a + c.sum();
            for vec in b {
                for any_array in vec {
                    let any_array = any_array.as_ref();
                    for any_path in any_array.iter() {
                        let any_path = any_path.as_ref();
                        total += any_path.iter().count();
                    }
                }
            }
            Ok(total)
        }
        };

        let after = transform_fn(before, &mut generic_gen_simple_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn complex_total<
            'any_nd_array4,
            AnyPath0: AsRef<std::path::Path>,
            AnyArray1: AsRef<[AnyPath0]>,
            AnyIter2: IntoIterator<Item = Vec<AnyArray1>>,
            AnyNdArray3: Into<ndarray::ArrayView1<'any_nd_array4, usize>>,
        >(
            a: usize,
            b: AnyIter2,
            c: AnyNdArray3,
        ) -> Result<usize, anyhow::Error> {
            let c = c.into();
            let b = b.into_iter();
            let mut total = a + c.sum();
            for vec in b {
                for any_array in vec {
                    let any_array = any_array.as_ref();
                    for any_path in any_array.iter() {
                        let any_path = any_path.as_ref();
                        total += any_path.iter().count();
                    }
                }
            }
            Ok(total)
        }

        assert_eq!(
            complex_total(17, [vec![["one"]]], [1, 2, 3].as_ref()).unwrap(),
            24
        );
    }

    #[test]
    fn doc_write() -> Result<(), anyhow::Error> {
        let before = parse_quote! {
        fn len_plus_2(s: AnyString) -> Result<usize, anyhow::Error> {
            Ok(s.len()+2)
        }        };
        let after = transform_fn(before, &mut generic_gen_simple_factory());
        println!("after: {}", quote! { #after});
        let expected = parse_quote! {
            fn len_plus_2<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
                let s = s.as_ref();
                Ok(s.len() + 2)
            }
        };
        assert_item_fn_eq(&after, &expected);

        fn len_plus_2<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            Ok(s.len() + 2)
        }

        assert_eq!(len_plus_2("hello")?, 7);

        Ok(())
    }

    #[test]
    #[should_panic(
        expected = "Expected at most one generic parameter, not '< AnyString0 , usize >'"
    )]
    fn one_bad_input() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyIter<AnyString,usize>) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    #[should_panic(expected = "Expected generic parameter to be a type, not '< 3 >'")]
    fn one_bad_input_2() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyIter<3>) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    #[should_panic(expected = "Expected <..> generic parameter,  not '(AnyString0)'")]
    fn one_bad_input_3() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyIter(AnyString)) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    #[should_panic(
        expected = "AnyArray expects a generic parameter, for example, AnyArray<usize> or AnyArray<AnyString>."
    )]
    fn one_bad_input_4() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyArray) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    #[should_panic(
        expected = "AnyIter expects a generic parameter, for example, AnyIter<usize> or AnyIter<AnyString>."
    )]
    fn one_bad_input_5() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyIter) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }
    #[test]
    #[should_panic(
        expected = "AnyNdArray expects a generic parameter, for example, AnyNdArray<usize> or AnyNdArray<AnyString>."
    )]
    fn one_bad_input_6() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyNdArray) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    #[should_panic(
        expected = "AnyString should not have a generic parameter, so AnyString, not AnyString<usize>."
    )]
    fn one_bad_input_7() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyString<usize>) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    #[should_panic(
        expected = "AnyPath should not have a generic parameter, so AnyPath, not AnyPath<usize>."
    )]
    fn one_bad_input_8() {
        let before = parse_quote! {
        pub fn any_str_len(s: AnyPath<usize>) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
           };
        let _after = transform_fn(before, &mut generic_gen_simple_factory());
    }

    #[test]
    fn see_bed_reader() {
        let before = parse_quote! {
         pub fn iid(mut self, iid: AnyIter<AnyString>) -> Self {
             // Unwrap will always work because BedBuilder starting with some metadata
             self.metadata.as_mut().unwrap().set_iid(iid);
             self
         }
        };
        let after = transform_fn(before, &mut generic_gen_simple_factory());
        println!("after: {}", quote! { #after});

        // pub fn iid<AnyString0: AsRef<str>, AnyIter1: IntoIterator<Item = AnyString0>>(
        //     mut self,
        //     iid: AnyIter1,
        // ) -> Self {
        //     let iid = iid.into_iter();
        //     self.metadata.as_mut().unwrap().set_iid(iid);
        //     self
        // }
    }

    // #[test]
    // #[should_panic(
    //     expected = "AnyNdArray expects a generic parameter, for example, AnyNdArray<usize> or AnyNdArray<AnyString>."
    // )]
    // fn one_bad_input_9() {
    //     let before = parse_quote! {
    //     pub fn any_str_len<'a>(s: AnyNdArray<'a,usize>) -> Result<usize, anyhow::Error> {
    //         let len = s.len();
    //         Ok(len)
    //     }
    //        };
    //     let _after = transform_fn(before, &mut generic_gen_simple_factory());
    // }
}
