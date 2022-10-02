// todo rename to input-special-derive (or derive-input-special)
// todo remove cargo stuff features of syn no longer needed.
// todo use AST spans test so that problems with the user's syntax are reported correctly
//           see quote_spanned! in https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs
// todo add nice error enum

// cmk Look more at https://github.com/dtolnay/syn/tree/master/examples/trace-var
// https://docs.rs/syn/latest/syn/fold/index.html#example
// cmk make nd support an optional feature

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
use uuid::Uuid;

// #[proc_macro_attribute]
pub fn input_special(_args: TokenStream, input: TokenStream) -> TokenStream {
    // panic!("input: {:#?}", &input);

    let old_item_fn = parse_macro_input!(input as ItemFn);
    // panic!("input: {:#?}", &input);

    let new_item_fn = transform_fn(old_item_fn, &mut UuidGenerator::new());

    TokenStream::from(quote!(#new_item_fn))
}

#[derive(Debug, Clone, EnumString)]
#[allow(clippy::enum_variant_names)]
enum Special {
    ArrayLike,
    StringLike,
    PathLike,
    IterLike,
    NdArrayLike,
}

impl Special {
    fn should_add_lifetime(&self) -> bool {
        match self {
            Special::ArrayLike | Special::StringLike | Special::PathLike | Special::IterLike => {
                false
            }
            Special::NdArrayLike => true,
        }
    }
    fn special_to_generic_param(
        &self,
        new_type: &TypePath,
        sub_type: Option<&Type>,
        lifetime: Option<Lifetime>,
    ) -> GenericParam {
        match &self {
            Special::ArrayLike => {
                let sub_type = sub_type.expect("array_1: sub_type");
                parse_quote!(#new_type : AsRef<[#sub_type]>)
            }
            Special::StringLike => {
                assert!(sub_type.is_none(), "string should not have sub_type"); // cmk will this get checked in release?
                parse_quote!(#new_type : AsRef<str>)
            }
            Special::PathLike => {
                assert!(sub_type.is_none(), "path should not have sub_type"); // cmk will this get checked in release?
                parse_quote!(#new_type : AsRef<std::path::Path>)
            }
            Special::IterLike => {
                let sub_type = sub_type.expect("iter_1: sub_type");
                parse_quote!(#new_type : IntoIterator<Item = #sub_type>)
            }
            Special::NdArrayLike => {
                let sub_type = sub_type.expect("nd_array: sub_type");
                // cmk on other branches, check is None
                let lifetime = lifetime.expect("nd_array: lifetime");
                parse_quote!(#new_type: Into<ndarray::ArrayView1<#lifetime, #sub_type>>)
            }
        }
    }

    fn pat_ident_to_stmt(&self, pat_ident: &PatIdent) -> Stmt {
        let name = &pat_ident.ident;
        match &self {
            Special::ArrayLike | Special::StringLike | Special::PathLike => {
                parse_quote! {
                    let #name = #name.as_ref();
                }
            }
            Special::IterLike => {
                parse_quote! {
                    let #name = #name.into_iter();
                }
            }
            Special::NdArrayLike => {
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

pub struct UuidGenerator {
    counter: usize,
    uuid: String,
}

impl UuidGenerator {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4().to_string().replace('-', "_"),
            counter: 0,
        }
    }
}

impl Iterator for UuidGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let s = format!("U{}_{}", self.uuid, self.counter);
        // cmk00 let result = parse_str(&s).expect("parse failure"); // cmk
        self.counter += 1;
        Some(s)
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

// Look for special inputs such as 's: StringLike'. If found, replace with generics special 's: S0'.
// Todo support: PathLike, IterLike<T>, ArrayLike<T> (including ArrayLike<PathLike>), NdArraySpecial<T>, etc.

// for each input, if it is top-level special, replace it with generic(s) and remember the generic(s) and the top-level variable.
// v: i32 -> v: i32, <>, {}
// v: StringLike -> v: S0, <S0: AsRef<str>>, {let v = v.as_ref();}
// v: IterLike<i32> -> v: S0, <S0: IntoIterator<Item = i32>>, {let v = v.into_iter();}
// v: IterLike<StringLike> -> v: S0, <S0: IntoIterator<Item = S1>, S1: AsRef<str>>, {let v = v.into_iter();}
// v: IterLike<IterLike<i32>> -> v: S0, <S0: IntoIterator<Item = S1>, S1: IntoIterator<Item = i32>>, {let v = v.into_iter();}
// v: IterLike<IterLike<StringLike>> -> v: S0, <S0: IntoIterator<Item = S1>, S1: IntoIterator<Item = S2>, S2: AsRef<str>>, {let v = v.into_iter();}
// v: [StringLike] -> v: [S0], <S0: AsRef<str>>, {}

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

// cmk see https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#destructuring-nested-structs-and-enums

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

impl Fold for DeltaPatType<'_> {
    fn fold_type_path(&mut self, type_path: TypePath) -> TypePath {
        // cmk println!("fold_type_path (before): {:?}", quote!(#type_path));

        // Search for any special (sub)subtypes, replacing them with generics.
        let mut type_path = fold_type_path(self, type_path);

        // If this top-level type is special, replace it with a generic.
        if let Some((segment, special)) = is_special_type_path(&type_path) {
            self.last_special = Some(special.clone()); // remember which kind of special found

            let s = self.generic_gen.next().unwrap(); // Generate the generic type, e.g. S23
            type_path = parse_str(&s).expect("parse failure");

            // Define the generic type, e.g. S23: AsRef<str>, and remember it.
            let sub_type = has_sub_type(segment.arguments); // Find anything inside angle brackets.

            let maybe_lifetime = if special.should_add_lifetime() {
                let s = self.generic_gen.next().unwrap().to_lowercase(); // Generate the generic type, e.g. S23
                let lifetime: Lifetime = parse_str(&format!("'{}", &s)).expect("parse failure"); // cmk 9 rules: best & easy way to create an object?
                let generic_param: GenericParam = parse_quote! { #lifetime };
                self.generic_params.push(generic_param);

                Some(lifetime)
                //GenericParam::Lifetime(LifetimeDef::new(lifetime)); // cmk 9 rules: This is another way to create an object.
            } else {
                None
            };

            let generic_param =
                special.special_to_generic_param(&type_path, sub_type.as_ref(), maybe_lifetime);
            self.generic_params.push(generic_param);
        } else {
            self.last_special = None;
        }
        // cmk println!("fold_type_path (after): {}", quote!(#type_path));
        type_path
    }
}

fn has_sub_type(args: PathArguments) -> Option<Type> {
    match args {
        PathArguments::None => None,
        PathArguments::AngleBracketed(ref args) => {
            let arg = first_and_only(args.args.iter()).expect("expected one argument cmk");
            // cmk println!("arg: {}", quote!(#arg));
            if let GenericArgument::Type(sub_type2) = arg {
                // cmk IterLike<PathLike>
                Some(sub_type2.clone())
            } else {
                panic!("expected GenericArgument::Type cmk");
            }
        }
        PathArguments::Parenthesized(_) => {
            panic!("Parenthesized not supported")
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
    // cmk 9 rules prettyplease::unparse vs quote! trick
    use crate::{transform_fn, DeltaPatType, UuidGenerator};
    use quote::quote;
    use syn::{fold::Fold, parse_quote, GenericParam, ItemFn, Lifetime};

    fn generic_gen_test_factory() -> impl Iterator<Item = String> + 'static {
        (0usize..).into_iter().map(|i| format!("S{i}"))
    }

    fn assert_item_fn_eq(after: &ItemFn, expected: &ItemFn) {
        if after == expected {
            return;
        }

        let after_str = format!("{}", quote!(#after));
        let expected_str = format!("{}", quote!(#expected));
        if after_str == expected_str {
            return;
        }
        println!("after: {}", after_str);
        println!("expected: {}", expected_str);
        panic!("after != expected");
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
        assert_item_fn_eq(&after, &expected);
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
            let b = b.as_ref();
            let a = a.as_ref();
            let len = a.len() + b.len();
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);
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
        assert_item_fn_eq(&after, &expected);
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
        assert_item_fn_eq(&after, &expected);
    }

    #[test]
    fn one_input_uuid() {
        let before = parse_quote! {pub fn any_str_len1(s: StringLike) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }};
        let _ = transform_fn(before, &mut generic_gen_test_factory());
    }

    #[test]
    fn one_path_input() {
        let before = parse_quote! {
        pub fn any_count_path(p: PathLike) -> Result<usize, anyhow::Error> {
            let count = p.iter().count();
            Ok(count)
        }        };
        let expected = parse_quote! {
        pub fn any_count_path<S0: AsRef<std::path::Path>>(p: S0) -> Result<usize, anyhow::Error> {
            let p = p.as_ref();
            let count = p.iter().count();
            Ok(count)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);
    }

    #[test]
    fn one_iter_usize_input() {
        let before = parse_quote! {
        pub fn any_count_iter(i: IterLike<usize>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }        };
        let expected = parse_quote! {
        pub fn any_count_iter<S0: IntoIterator<Item = usize>>(i: S0) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<S0: IntoIterator<Item = usize>>(
            i: S0,
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    }

    #[test]
    fn one_iter_i32() {
        let before = parse_quote! {
        pub fn any_count_iter(i: IterLike<i32>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }        };
        let expected = parse_quote! {
        pub fn any_count_iter<S0: IntoIterator<Item = i32>>(i: S0) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<S0: IntoIterator<Item = i32>>(i: S0) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    }

    #[test]
    fn one_iter_t() {
        let before = parse_quote! {
        pub fn any_count_iter<T>(i: IterLike<T>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }        };
        let expected = parse_quote! {
        pub fn any_count_iter<T, S0: IntoIterator<Item = T>>(i: S0) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<T, S0: IntoIterator<Item = T>>(
            i: S0,
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
        pub fn any_count_iter(i: IterLike<PathLike>) -> Result<usize, anyhow::Error> {
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }        };
        let expected = parse_quote! {
        pub fn any_count_iter<S0: AsRef<std::path::Path>, S1: IntoIterator<Item = S0>>(
            i: S1
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter(); // todo should the map be optional?
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_iter<S0: AsRef<std::path::Path>, S1: IntoIterator<Item = S0>>(
            i: S1,
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter(); // todo should the map be optional?
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_iter(["a/b", "d"]).unwrap(), 3);
    }

    #[test]
    fn one_vec_path() {
        let before = parse_quote! {
        pub fn any_count_vec(
            i: Vec<PathLike>,
        ) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }};
        let expected = parse_quote! {
        pub fn any_count_vec<S0: AsRef<std::path::Path>>(
            i: Vec<S0>
        ) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_count_vec<S0: AsRef<std::path::Path>>(
            i: Vec<S0>,
        ) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
    }

    #[test]
    fn fold_one_path() {
        // cmk 9 rules: parse_quote!
        // cmk 9 rules: use format!(quote!()) to generate strings of code
        // cmk 9 rules quote! is a nice way to display short ASTs on one line, too
        let before = parse_quote! {IterLike<PathLike> };
        println!("before: {}", quote!(before));
        let mut gen = generic_gen_test_factory();
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
        pub fn any_slice_len(a: ArrayLike<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }        };
        let expected = parse_quote! {
        pub fn any_slice_len<S0: AsRef<[usize]>>(a: S0) -> Result<usize, anyhow::Error> {
            let a = a.as_ref();
            let len = a.len();
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_slice_len<S0: AsRef<[usize]>>(a: S0) -> Result<usize, anyhow::Error> {
            let a = a.as_ref();
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_slice_len([1, 2, 3]).unwrap(), 3);
    }

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

    #[test]
    fn one_ndarray_usize_input() {
        let before = parse_quote! {
        pub fn any_slice_len(a: NdArrayLike<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }        };
        let expected = parse_quote! {
        pub fn any_slice_len<'s1, S0: Into<ndarray::ArrayView1<'s1, usize>>>(
            a: S0
        ) -> Result<usize, anyhow::Error> {
            let a = a.into();
            let len = a.len();
            Ok(len)
        }};

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        // cmk clippy would like a comma after a:S0, but the macro doesn't do that.
        pub fn any_slice_len<'s1, S0: Into<ndarray::ArrayView1<'s1, usize>>>(
            a: S0,
        ) -> Result<usize, anyhow::Error> {
            let a = a.into();
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_slice_len([1, 2, 3].as_ref()).unwrap(), 3);
    }
}
