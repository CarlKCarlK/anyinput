#![doc = include_str!("../README.md")]

mod tests;
// todo use AST spans test so that problems with the user's syntax are reported correctly
//           see quote_spanned! in https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs

use proc_macro2::Span;
use proc_macro_error::abort;
// todo later could nested .as_ref(), .into_iter(), and .into() be replaced with a single method or macro?
use std::str::FromStr;
use strum::EnumString;
use syn::fold::{fold_type_path, Fold};
use syn::{
    parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg, GenericArgument,
    GenericParam, Generics, ItemFn, Lifetime, Pat, PatIdent, PatType, PathArguments, PathSegment,
    Signature, Stmt, Type, TypePath,
};
pub fn generic_gen_simple_factory() -> impl Iterator<Item = String> + 'static {
    (0usize..).into_iter().map(|i| format!("{i}"))
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

// todo later do something interesting with 2d ndarray/views

impl Special {
    fn should_add_lifetime(&self) -> bool {
        match self {
            Special::AnyArray | Special::AnyString | Special::AnyPath | Special::AnyIter => false,
            Special::AnyNdArray => true,
        }
    }

    fn special_to_generic_param(
        &self,
        old_type: &TypePath,
        new_type: &TypePath,
        sub_type: Option<&Type>,
        lifetime: Option<Lifetime>,
    ) -> GenericParam {
        match &self {
            Special::AnyString => {
                if sub_type.is_some() {
                    abort!(old_type,"AnyString should not have a generic parameter, so 'AnyString', not 'AnyString<_>'.")
                };
                assert!(lifetime.is_none(), "AnyString should not have a lifetime.");
                parse_quote!(#new_type : AsRef<str>)
            }
            Special::AnyPath => {
                if sub_type.is_some() {
                    abort!(old_type,"AnyPath should not have a generic parameter, so 'AnyPath', not 'AnyPath<_>'.")
                };
                assert!(lifetime.is_none(), "AnyPath should not have a lifetime.");
                parse_quote!(#new_type : AsRef<std::path::Path>)
            }
            Special::AnyArray => {
                let sub_type = match sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(old_type,"AnyArray expects a generic parameter, for example, AnyArray<usize> or AnyArray<AnyString>.")
                    }
                };
                assert!(lifetime.is_none(), "AnyArray should not have a lifetime.");
                parse_quote!(#new_type : AsRef<[#sub_type]>)
            }
            Special::AnyIter => {
                let sub_type = match sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(old_type,"AnyIter expects a generic parameter, for example, AnyIter<usize> or AnyIter<AnyString>.")
                    }
                };
                assert!(lifetime.is_none(), "AnyIter should not have a lifetime.");
                parse_quote!(#new_type : IntoIterator<Item = #sub_type>)
            }
            Special::AnyNdArray => {
                match sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(old_type,"AnyNdArray expects a generic parameter, for example, AnyNdArray<usize> or AnyNdArray<AnyString>.")
                    }
                };
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
    // // todo Is this the best way to create a new function from an old one?
    let span = Span::call_site();
    ItemFn {
        sig: Signature {
            generics: Generics {
                lt_token: Some(syn::Token![<]([span])),
                params: delta_fun_args.generic_params,
                gt_token: Some(syn::Token![>]([span])),
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

fn first_and_only<T, I: Iterator<Item = T>>(mut iter: I) -> Option<T> {
    let first = iter.next()?;
    if iter.next().is_some() {
        None
    } else {
        Some(first)
    }
}

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
    fn fold_type_path(&mut self, type_path_original: TypePath) -> TypePath {
        // println!("fold_type_path (before): {:?}", quote!(#type_path));

        // Search for any special (sub)subtypes, replacing them with generics.
        let mut type_path = fold_type_path(self, type_path_original.clone());

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
            let sub_type = has_sub_type(&type_path_original, segment.arguments); // Find anything inside angle brackets.

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

            let generic_param = special.special_to_generic_param(
                &type_path_original,
                &type_path,
                sub_type.as_ref(),
                maybe_lifetime,
            );
            self.generic_params.push(generic_param);
        } else {
            self.last_special = None;
        }
        // println!("fold_type_path (after): {}", quote!(#type_path));
        type_path
    }
}

fn has_sub_type(old_type: &TypePath, args: PathArguments) -> Option<Type> {
    match args {
        PathArguments::None => None,
        PathArguments::AngleBracketed(ref args) => {
            let arg = first_and_only(args.args.iter())
                .unwrap_or_else(|| abort!(old_type, "Expected at exactly one generic parameter."));
            // println!("arg: {}", quote!(#arg));
            if let GenericArgument::Type(sub_type2) = arg {
                Some(sub_type2.clone())
            } else {
                abort!(old_type, "Expected generic parameter to be a type.")
            }
        }
        PathArguments::Parenthesized(_) => {
            abort!(old_type, "Expected <..> generic parameter.")
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
