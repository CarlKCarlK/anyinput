#![doc = include_str!("../README.md")]

mod tests;

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use std::str::FromStr;
use strum::EnumString;
use syn::fold::Fold;
use syn::Ident;
use syn::{
    parse2, parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg,
    GenericArgument, GenericParam, Generics, ItemFn, Lifetime, Pat, PatIdent, PatType,
    PathArguments, PathSegment, Signature, Stmt, Type, TypePath,
};

pub fn anyinput_core(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        abort!(args, "anyinput does not take any arguments.")
    }

    // proc_marco2 version of "parse_macro_input!(input as ItemFn)"
    let old_item_fn = match parse2::<ItemFn>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(error) => return error.to_compile_error(),
    };

    let new_item_fn = transform_fn(old_item_fn);

    quote!(#new_item_fn)
}

fn transform_fn(old_fn: ItemFn) -> ItemFn {
    let mut suffix_iter = simple_suffix_iter_factory();

    // Start with 1. no function arguments, 2. the old function's generics, 3. the old function's statements
    let init = DeltaFnArgList {
        fn_args: Punctuated::<FnArg, Comma>::new(),
        generic_params: old_fn.sig.generics.params.clone(),
        stmts: old_fn.block.stmts,
    };

    // Transform each old argument of the function, accumulating: the new argument, new generic definitions and new statements
    let delta_fun_arg_list = (old_fn.sig.inputs)
        .iter()
        .map(|old_fn_arg| transform_fn_arg(old_fn_arg, &mut suffix_iter))
        .fold(init, |mut delta_fun_arg_list, delta_fun_arg| {
            delta_fun_arg_list.merge(delta_fun_arg);
            delta_fun_arg_list
        });

    // Create a new function with the transformed inputs and accumulated generic definitions, and statements.
    // Use Rust's struct update syntax (https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/)
    ItemFn {
        sig: Signature {
            generics: Generics {
                lt_token: parse_quote!(<),
                params: delta_fun_arg_list.generic_params,
                gt_token: parse_quote!(>),
                ..old_fn.sig.generics.clone()
            },
            inputs: delta_fun_arg_list.fn_args,
            ..old_fn.sig.clone()
        },
        block: Box::new(Block {
            stmts: delta_fun_arg_list.stmts,
            ..*old_fn.block
        }),
        ..old_fn
    }
}

struct DeltaFnArgList {
    fn_args: Punctuated<FnArg, Comma>,
    generic_params: Punctuated<GenericParam, Comma>,
    stmts: Vec<Stmt>,
}

impl DeltaFnArgList {
    fn merge(&mut self, delta_fn_arg: DeltaFnArg) {
        self.fn_args.push(delta_fn_arg.fn_arg);
        self.generic_params.extend(delta_fn_arg.generic_params);
        for (index, stmt) in delta_fn_arg.stmts.into_iter().enumerate() {
            self.stmts.insert(index, stmt);
        }
    }
}

// Define a generator for suffixes of generic types. "0", "1", "2", ...
// This is used to create unique names for generic types.
// Could switch to one based on UUIDs, but this is easier to read.
fn simple_suffix_iter_factory() -> impl Iterator<Item = String> + 'static {
    (0usize..).into_iter().map(|i| format!("{i}"))
}

// Define the Specials and their properties.
#[derive(Debug, Clone, EnumString)]
#[allow(clippy::enum_variant_names)]
enum Special {
    AnyArray,
    AnyString,
    AnyPath,
    AnyIter,
    AnyNdArray,
}

impl Special {
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

    fn ident_to_stmt(&self, name: &Ident) -> Stmt {
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

    fn should_add_lifetime(&self) -> bool {
        match self {
            Special::AnyArray | Special::AnyString | Special::AnyPath | Special::AnyIter => false,
            Special::AnyNdArray => true,
        }
    }
}

// If a function argument contains a special type(s), re-write it.
fn transform_fn_arg(
    old_fn_arg: &FnArg,
    suffix_iter: &mut impl Iterator<Item = String>,
) -> DeltaFnArg {
    // If the function input is normal (not self, not a macro, etc) ...
    if let Some((pat_ident, pat_type)) = is_normal_fn_arg(old_fn_arg) {
        // Replace any specials in the type with generics.
        let (delta_pat_type, new_pat_type) = replace_any_specials(pat_type.clone(), suffix_iter);

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

#[derive(Debug)]
// The new function input, any statements to add, and any new generic definitions.
struct DeltaFnArg {
    fn_arg: FnArg,
    generic_params: Vec<GenericParam>,
    stmts: Vec<Stmt>,
}

impl DeltaPatType<'_> {
    fn generate_any_stmts(&self, pat_ident: &PatIdent) -> Vec<Stmt> {
        // If the top-level type is a special, add a statement to convert
        // from its generic type to to a concrete type.
        // For example,  "let x = x.into_iter();" for AnyIter.
        if let Some(special) = &self.last_special {
            vec![special.ident_to_stmt(&pat_ident.ident)]
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

// Search type and its (sub)subtypes for specials starting at the deepest level.
// When one is found, replace it with a generic.
// Finally, return the new type and a list of the generic definitions.
// Also, if the top-level type was special, return the special type.
#[allow(clippy::ptr_arg)]
fn replace_any_specials(
    old_pat_type: PatType,
    suffix_iter: &mut impl Iterator<Item = String>,
) -> (DeltaPatType, PatType) {
    let mut delta_pat_type = DeltaPatType {
        generic_params: vec![],
        suffix_iter,
        last_special: None,
    };
    let new_path_type = delta_pat_type.fold_pat_type(old_pat_type);

    (delta_pat_type, new_path_type)
}

struct DeltaPatType<'a> {
    generic_params: Vec<GenericParam>,
    suffix_iter: &'a mut dyn Iterator<Item = String>,
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
    fn fold_type_path(&mut self, type_path_old: TypePath) -> TypePath {
        // println!("fold_type_path (before) code: {}", quote!(#type_path_old));
        // println!("                      syntax: {:?}", type_path_old);

        // Search for any special (sub)subtypes, replacing them with generics.
        let type_path_middle = syn::fold::fold_type_path(self, type_path_old.clone());

        // If this top-level type is special, replace it with a generic.
        if let Some((segment, special)) = is_special_type_path(&type_path_middle) {
            self.last_special = Some(special.clone()); // remember which kind of special found

            let suffix = self
                .suffix_iter
                .next()
                .expect("Internal error: ran out of generic suffixes");
            let generic_name = format!("{:?}{}", &special, suffix); // todo implement display and remove "?"
            let type_path_new =
                parse_str(&generic_name).expect("Internal error: failed to parse generic name");

            // Define the generic type, e.g. S23: AsRef<str>, and remember it.
            let sub_type = has_sub_type(&type_path_old, segment.arguments); // Find anything inside angle brackets.

            let maybe_lifetime = if special.should_add_lifetime() {
                let suffix = &self
                    .suffix_iter
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
                &type_path_old,
                &type_path_new,
                sub_type.as_ref(),
                maybe_lifetime,
            );
            self.generic_params.push(generic_param);
            println!("fold_type_path (after) code: {}", quote!(#type_path_new));
            println!("                      syntax: {:?}", type_path_new);
            type_path_new
        } else {
            self.last_special = None;
            println!("fold_type_path (after) code: {}", quote!(#type_path_middle));
            println!("                      syntax: {:?}", type_path_middle);
            type_path_middle
        }
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
    //cmk be sure it doesn't have a qself https://docs.rs/syn/latest/syn/struct.TypePath.html#
    //cmk is this the best way to check this or could use a match?
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
// Utility that tells if an iterator contains exactly one element.
fn first_and_only<T, I: Iterator<Item = T>>(mut iter: I) -> Option<T> {
    let first = iter.next()?;
    if iter.next().is_some() {
        None
    } else {
        Some(first)
    }
}

// todo later could nested .as_ref(), .into_iter(), and .into() be replaced with a single method or macro?
// todo later do something interesting with 2d ndarray/views
