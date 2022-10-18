#![doc = include_str!("../README.md")]

mod tests;

use proc_macro2::TokenStream;
use proc_macro_error::{abort, SpanRange};
use quote::quote;
use std::str::FromStr;
use strum::{Display, EnumString};
use syn::fold::Fold;
use syn::WhereClause;
use syn::{
    parse2, parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg,
    GenericArgument, GenericParam, Generics, Ident, ItemFn, Lifetime, Pat, PatIdent, PatType,
    PathArguments, Signature, Stmt, Type, TypePath, WherePredicate,
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

pub fn anyinput_core_sample(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        abort!(args, "anyinput does not take any arguments.")
    }

    // proc_marco2 version of "parse_macro_input!(input as ItemFn)"
    let old_item_fn = match parse2::<ItemFn>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(error) => return error.to_compile_error(),
    };

    let new_item_fn = transform_fn_sample(old_item_fn);

    quote!(#new_item_fn)
}

fn transform_fn_sample(_item_fn: ItemFn) -> ItemFn {
    parse_quote! {
        fn hello_world() {
            println!("Hello, world!");
        }
    }
}

fn transform_fn(item_fn: ItemFn) -> ItemFn {
    let mut suffix_iter = simple_suffix_iter_factory();
    let delta_fn_arg_new = |fn_arg| DeltaFnArg::new(fn_arg, &mut suffix_iter);

    // Transform each old argument of the function, accumulating: the new argument, new generics, wheres, and statements
    // Then, turn the accumulation into a new function.
    item_fn
        .sig
        .inputs
        .iter()
        .map(delta_fn_arg_new)
        .fold(ItemFnAcc::init(&item_fn), ItemFnAcc::fold)
        .to_item_fn()
}

struct ItemFnAcc<'a> {
    old_fn: &'a ItemFn,
    fn_args: Punctuated<FnArg, Comma>,
    generic_params: Punctuated<GenericParam, Comma>,
    where_predicates: Punctuated<WherePredicate, Comma>,
    stmts: Vec<Stmt>,
}

impl ItemFnAcc<'_> {
    fn init(item_fn: &ItemFn) -> ItemFnAcc {
        // Start with 1. no function arguments, 2. the old function's generics, wheres, and statements
        ItemFnAcc {
            old_fn: item_fn,
            fn_args: Punctuated::<FnArg, Comma>::new(),
            generic_params: item_fn.sig.generics.params.clone(),
            where_predicates: ItemFnAcc::extract_where_predicates(item_fn),
            stmts: item_fn.block.stmts.clone(),
        }
    }

    // Even if the where clause is None, we still need to return an empty Punctuated
    fn extract_where_predicates(item_fn: &ItemFn) -> Punctuated<WherePredicate, Comma> {
        if let Some(WhereClause { predicates, .. }) = &item_fn.sig.generics.where_clause {
            predicates.clone()
        } else {
            parse_quote!()
        }
    }

    fn fold(mut self, delta: DeltaFnArg) -> Self {
        self.fn_args.push(delta.fn_arg);
        self.generic_params.extend(delta.generic_params);
        self.where_predicates.extend(delta.where_predicates);
        for (index, element) in delta.stmt.into_iter().enumerate() {
            self.stmts.insert(index, element);
        }
        self
    }

    // Use Rust's struct update syntax (https://www.reddit.com/r/rust/comments/pchp8h/media_struct_update_syntax_in_rust/)
    fn to_item_fn(&self) -> ItemFn {
        ItemFn {
            sig: Signature {
                generics: self.to_generics(),
                inputs: self.fn_args.clone(),
                ..self.old_fn.sig.clone()
            },
            block: Box::new(Block {
                stmts: self.stmts.clone(),
                ..*self.old_fn.block.clone()
            }),
            ..self.old_fn.clone()
        }
    }

    fn to_generics(&self) -> Generics {
        Generics {
            lt_token: parse_quote!(<),
            params: self.generic_params.clone(),
            gt_token: parse_quote!(>),
            where_clause: self.to_where_clause(),
        }
    }

    fn to_where_clause(&self) -> Option<WhereClause> {
        if self.where_predicates.is_empty() {
            None
        } else {
            Some(WhereClause {
                where_token: parse_quote!(where),
                predicates: self.where_predicates.clone(),
            })
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
    fn special_to_where_predicate(
        &self,
        generic: &TypePath, // for example: AnyArray0
        maybe_sub_type: Option<Type>,
        maybe_lifetime: Option<Lifetime>,
        span_range: &SpanRange,
    ) -> WherePredicate {
        match &self {
            Special::AnyString => {
                if maybe_sub_type.is_some() {
                    abort!(span_range,"AnyString should not have a generic parameter, so 'AnyString', not 'AnyString<_>'.")
                };
                assert!(
                    maybe_lifetime.is_none(),
                    "AnyString should not have a lifetime."
                );
                parse_quote! {
                    #generic : AsRef<str>
                }
            }
            Special::AnyPath => {
                if maybe_sub_type.is_some() {
                    abort!(span_range,"AnyPath should not have a generic parameter, so 'AnyPath', not 'AnyPath<_>'.")
                };
                assert!(
                    maybe_lifetime.is_none(),
                    "AnyPath should not have a lifetime."
                );
                parse_quote! {
                    #generic : AsRef<std::path::Path>
                }
            }
            Special::AnyArray => {
                let sub_type = match maybe_sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(span_range,"AnyArray expects a generic parameter, for example, AnyArray<usize> or AnyArray<AnyString>.")
                    }
                };
                assert!(
                    // cmk change to abort
                    maybe_lifetime.is_none(),
                    "AnyArray should not have a lifetime."
                );
                parse_quote! {
                    #generic : AsRef<[#sub_type]>
                }
            }
            Special::AnyIter => {
                let sub_type = match maybe_sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(span_range,"AnyIter expects a generic parameter, for example, AnyIter<usize> or AnyIter<AnyString>.")
                    }
                };
                assert!(
                    // cmk change to abort
                    maybe_lifetime.is_none(),
                    "AnyIter should not have a lifetime."
                );
                parse_quote! {
                    #generic : IntoIterator<Item = #sub_type>
                }
            }
            Special::AnyNdArray => {
                let sub_type = match maybe_sub_type {
                    Some(sub_type) => sub_type,
                    None => {
                        abort!(span_range,"AnyNdArray expects a generic parameter, for example, AnyNdArray<usize> or AnyNdArray<AnyString>.")
                    }
                };
                let lifetime =
                    maybe_lifetime.expect("Internal error: AnyNdArray should be given a lifetime.");
                parse_quote! {
                    #generic: Into<ndarray::ArrayView1<#lifetime, #sub_type>>
                }
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

#[derive(Debug)]
// The new function input, any statements to add, and any new generic definitions.
struct DeltaFnArg {
    fn_arg: FnArg,
    generic_params: Vec<GenericParam>,
    where_predicates: Vec<WherePredicate>,
    stmt: Option<Stmt>,
}

impl DeltaFnArg {
    // If a function argument contains a special type(s), re-write it/them.
    fn new(fn_arg: &FnArg, suffix_iter: &mut impl Iterator<Item = String>) -> DeltaFnArg {
        // If the function input is normal (not self, not a macro, etc) ...
        if let Some((pat_ident, pat_type)) = DeltaFnArg::is_normal_fn_arg(fn_arg) {
            // Replace any specials in the type with generics.
            DeltaFnArg::replace_any_specials(pat_type.clone(), pat_ident, suffix_iter)
        } else {
            // if input is not normal, return it unchanged.
            DeltaFnArg {
                fn_arg: fn_arg.clone(),
                generic_params: vec![],
                where_predicates: vec![],
                stmt: None,
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
        pat_ident: &PatIdent,
        suffix_iter: &mut impl Iterator<Item = String>,
    ) -> DeltaFnArg {
        let mut delta_pat_type = DeltaPatType::new(suffix_iter);
        let new_pat_type = delta_pat_type.fold_pat_type(old_pat_type);

        // Return the new function input, any statements to add, and any new generic definitions.
        DeltaFnArg {
            fn_arg: FnArg::Typed(new_pat_type),
            stmt: delta_pat_type.generate_any_stmt(pat_ident),
            generic_params: delta_pat_type.generic_params,
            where_predicates: delta_pat_type.where_predicates,
        }
    }
}

struct DeltaPatType<'a> {
    generic_params: Vec<GenericParam>,
    where_predicates: Vec<WherePredicate>,
    suffix_iter: &'a mut dyn Iterator<Item = String>,
    last_special: Option<Special>,
}

impl Fold for DeltaPatType<'_> {
    fn fold_type_path(&mut self, type_path_old: TypePath) -> TypePath {
        let span_range = SpanRange::from_tokens(&type_path_old); // used by abort!

        // Apply "fold" recursively to process specials in subtypes, for example, Vec<AnyString>.
        let type_path_middle = syn::fold::fold_type_path(self, type_path_old);

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
    fn new(suffix_iter: &'a mut dyn Iterator<Item = String>) -> Self {
        DeltaPatType {
            generic_params: vec![],
            where_predicates: vec![],
            suffix_iter,
            last_special: None,
        }
    }

    // If the top-level type is a special, add a statement to convert
    // from its generic type to to a concrete type.
    // For example,  "let x = x.into_iter();" for AnyIter.
    fn generate_any_stmt(&self, pat_ident: &PatIdent) -> Option<Stmt> {
        if let Some(special) = &self.last_special {
            let stmt = special.ident_to_stmt(&pat_ident.ident);
            Some(stmt)
        } else {
            None
        }
    }

    // Define the generic type, for example, "AnyString3: AsRef<str>", and remember the definition.
    fn create_and_define_generic(
        &mut self,
        special: Special,
        maybe_sub_type: Option<Type>,
        span_range: &SpanRange,
    ) -> TypePath {
        let generic = self.create_generic(&special); // for example, "AnyString3"
        let maybe_lifetime = self.create_maybe_lifetime(&special);
        let where_predicate = special.special_to_where_predicate(
            &generic,
            maybe_sub_type,
            maybe_lifetime,
            span_range,
        );
        let generic_param: GenericParam = parse_quote!(#generic);
        self.generic_params.push(generic_param);
        self.where_predicates.push(where_predicate);
        generic
    }

    // create a lifetime if needed, for example, Some('any_nd_array_3) or None
    fn create_maybe_lifetime(&mut self, special: &Special) -> Option<Lifetime> {
        if special.should_add_lifetime() {
            let lifetime = self.create_lifetime(special);
            let generic_param: GenericParam = parse_quote!(#lifetime);
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
// todo later when does the std lib use where clauses? Is there an informal rule? Should there be an option?
