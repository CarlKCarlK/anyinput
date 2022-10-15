#![doc = include_str!("../README.md")]

mod tests;

use proc_macro2::TokenStream;
use proc_macro_error::{abort, SpanRange};
use quote::quote;
use std::str::FromStr;
use strum::{Display, EnumString};
use syn::fold::Fold;
use syn::Ident;
use syn::{
    parse2, parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg,
    GenericArgument, GenericParam, Generics, ItemFn, Lifetime, Pat, PatIdent, PatType,
    PathArguments, Signature, Stmt, Type, TypePath,
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
        for (index, stmt) in delta_fn_arg.stmt.into_iter().enumerate() {
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
#[derive(Debug, Clone, EnumString, Display)]
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
        generic: &TypePath, // for example: AnyArray0
        maybe_sub_type: Option<Type>,
        maybe_lifetime: Option<Lifetime>,
        span: &SpanRange,
    ) -> GenericParam {
        match &self {
            Special::AnyString => {
                if maybe_sub_type.is_some() {
                    abort!(span,"AnyString should not have a generic parameter, so 'AnyString', not 'AnyString<_>'.")
                };
                assert!(
                    maybe_lifetime.is_none(),
                    "AnyString should not have a lifetime."
                );
                parse_quote!(#generic : AsRef<str>)
            }
            Special::AnyPath => {
                if maybe_sub_type.is_some() {
                    abort!(span,"AnyPath should not have a generic parameter, so 'AnyPath', not 'AnyPath<_>'.")
                };
                assert!(
                    maybe_lifetime.is_none(),
                    "AnyPath should not have a lifetime."
                );
                parse_quote!(#generic : AsRef<std::path::Path>)
            }
            Special::AnyArray => {
                let sub_type = match maybe_sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(span,"AnyArray expects a generic parameter, for example, AnyArray<usize> or AnyArray<AnyString>.")
                    }
                };
                assert!(
                    maybe_lifetime.is_none(),
                    "AnyArray should not have a lifetime."
                );
                parse_quote!(#generic : AsRef<[#sub_type]>)
            }
            Special::AnyIter => {
                let sub_type = match maybe_sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(span,"AnyIter expects a generic parameter, for example, AnyIter<usize> or AnyIter<AnyString>.")
                    }
                };
                assert!(
                    maybe_lifetime.is_none(),
                    "AnyIter should not have a lifetime."
                );
                parse_quote!(#generic : IntoIterator<Item = #sub_type>)
            }
            Special::AnyNdArray => {
                let sub_type = match maybe_sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(span,"AnyNdArray expects a generic parameter, for example, AnyNdArray<usize> or AnyNdArray<AnyString>.")
                    }
                };
                let lifetime =
                    maybe_lifetime.expect("Internal error: AnyNdArray should be given a lifetime.");
                parse_quote!(#generic: Into<ndarray::ArrayView1<#lifetime, #sub_type>>)
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

    fn maybe_new(type_path: &TypePath, span_range: &SpanRange) -> Option<(Special, Option<Type>)> {
        // A special type path has exactly one segment and a name from the Special enum.
        if type_path.qself.is_none() {
            if let Some(segment) = first_and_only(type_path.path.segments.iter()) {
                if let Ok(special) = Special::from_str(segment.ident.to_string().as_ref()) {
                    let maybe_sub_type =
                        Special::create_maybe_sub_type(&segment.arguments, span_range);
                    return Some((special, maybe_sub_type));
                }
            }
        }
        None
    }

    fn create_maybe_sub_type(args: &PathArguments, span_range: &SpanRange) -> Option<Type> {
        match args {
            PathArguments::None => None,
            PathArguments::AngleBracketed(ref args) => {
                let arg = first_and_only(args.args.iter()).unwrap_or_else(|| {
                    abort!(span_range, "Expected at exactly one generic parameter.")
                });
                // println!("arg: {}", quote!(#arg));
                if let GenericArgument::Type(sub_type2) = arg {
                    Some(sub_type2.clone())
                } else {
                    abort!(span_range, "Expected generic parameter to be a type.")
                }
            }
            PathArguments::Parenthesized(_) => {
                abort!(span_range, "Expected <..> generic parameter.")
            }
        }
    }

    // Utility that turns camel case into snake case.
    // For example, "AnyString" -> "any_string".
    fn to_snake_case(&self) -> String {
        let mut snake_case_string = String::new();
        for (index, ch) in self.to_string().chars().enumerate() {
            if index > 0 && ch.is_uppercase() {
                snake_case_string.push('_');
            }
            snake_case_string.push(ch.to_ascii_lowercase());
        }
        snake_case_string
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
            stmt: delta_pat_type.generate_any_stmt(pat_ident),
            generic_params: delta_pat_type.generic_params,
        }
    } else {
        // if input is not normal, return it unchanged.
        DeltaFnArg {
            fn_arg: old_fn_arg.clone(),
            generic_params: vec![],
            stmt: None,
        }
    }
}

#[derive(Debug)]
// The new function input, any statements to add, and any new generic definitions.
struct DeltaFnArg {
    fn_arg: FnArg,
    generic_params: Vec<GenericParam>,
    stmt: Option<Stmt>,
}

impl DeltaPatType<'_> {
    fn generate_any_stmt(&self, pat_ident: &PatIdent) -> Option<Stmt> {
        // If the top-level type is a special, add a statement to convert
        // from its generic type to to a concrete type.
        // For example,  "let x = x.into_iter();" for AnyIter.
        if let Some(special) = &self.last_special {
            let stmt = special.ident_to_stmt(&pat_ident.ident);
            Some(stmt)
        } else {
            None
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

impl Fold for DeltaPatType<'_> {
    fn fold_type_path(&mut self, type_path_old: TypePath) -> TypePath {
        // Apply "fold" recursively to process specials in subtypes, for example, Vec<AnyString>.
        let type_path_middle = syn::fold::fold_type_path(self, type_path_old);

        let span_range = SpanRange::from_tokens(&type_path_middle); // used by abort!

        // If this type is special, replace it with a generic.
        if let Some((special, maybe_sub_types)) = Special::maybe_new(&type_path_middle, &span_range)
        {
            self.last_special = Some(special.clone()); // remember the special found (used for stmt generation)
            self.create_and_define_generic(special, maybe_sub_types, &span_range)
        } else {
            self.last_special = None;
            type_path_middle
        }
    }
}

impl<'a> DeltaPatType<'a> {
    // Define the generic type, for example, "AnyString3: AsRef<str>", and remember the definition.
    fn create_and_define_generic(
        &mut self,
        special: Special,
        maybe_sub_type: Option<Type>,
        span_range: &SpanRange,
    ) -> TypePath {
        let generic = self.create_generic(&special); // for example, "AnyString3"
        let maybe_lifetime = self.create_maybe_lifetime(&special);
        let generic_param =
            special.special_to_generic_param(&generic, maybe_sub_type, maybe_lifetime, span_range);
        self.generic_params.push(generic_param);
        generic
    }

    // create a lifetime if needed, for example, Some('any_nd_array_3) or None
    fn create_maybe_lifetime(&mut self, special: &Special) -> Option<Lifetime> {
        if special.should_add_lifetime() {
            let lifetime = self.create_lifetime(special);
            let generic_param: GenericParam = parse_quote! { #lifetime };
            self.generic_params.push(generic_param);

            Some(lifetime)
        } else {
            None
        }
    }

    // Create a new generic type, for example, "AnyString3"
    fn create_generic(&mut self, special: &Special) -> TypePath {
        let suffix = self.create_suffix();
        let generic_name = format!("{}{}", &special, suffix);
        parse_str(&generic_name).expect("Internal error: failed to parse generic name")
    }

    // Create a new lifetime, for example, "'any_nd_array_4"
    fn create_lifetime(&mut self, special: &Special) -> Lifetime {
        let lifetime_name = format!("'{}{}", special.to_snake_case(), self.create_suffix());
        parse_str(&lifetime_name).expect("Internal error: failed to parse lifetime name")
    }

    // Create a new suffix, for example, "4"
    fn create_suffix(&mut self) -> String {
        self.suffix_iter
            .next()
            .expect("Internal error: ran out of generic suffixes")
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
