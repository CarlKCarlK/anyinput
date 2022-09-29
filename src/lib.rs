// todo rename to input-special-derive (or derive-input-special)
// todo remove cargo stuff features of syn no longer needed.
// todo use AST spans test so that problems with the user's syntax are reported correctly
//           see quote_spanned! in https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs
// todo add nice error enum

// cmk Look more at https://github.com/dtolnay/syn/tree/master/examples/trace-var
// https://docs.rs/syn/latest/syn/fold/index.html#example
// cmk what about Vec<StringLike>?
// cmk add nd::array view
// cmk make nd support an optional feature

use std::str::FromStr;

use quote::quote;
use strum::EnumString;
use syn::__private::TokenStream;
use syn::fold::{fold_type_path, Fold};
// todo don't use private
use syn::{
    parse_macro_input, parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg,
    GenericArgument, GenericParam, Generics, ItemFn, Pat, PatIdent, PatType, PathArguments,
    PathSegment, Signature, Stmt, Type, TypePath,
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
}

impl Special {
    fn special_to_generic_param(
        &self,
        new_type: &TypePath,
        sub_type: Option<&Type>,
    ) -> GenericParam {
        match &self {
            Special::ArrayLike => {
                let sub_type = sub_type.expect("array_1: sub_type");
                parse_quote!(#new_type : AsRef<[#sub_type]>)
            }
            Special::StringLike => {
                assert!(sub_type.is_none(), "string should not have sub_type"); // cmk will this get check in release?
                parse_quote!(#new_type : AsRef<str>)
            }
            Special::PathLike => {
                assert!(sub_type.is_none(), "path should not have sub_type"); // cmk will this get check in release?
                parse_quote!(#new_type : AsRef<std::path::Path>)
            }
            Special::IterLike => {
                let sub_type = sub_type.expect("iter_1: sub_type");
                parse_quote!(#new_type : IntoIterator<Item = #sub_type>)
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
        }
    }
}

pub fn transform_fn(old_fn: ItemFn, generic_gen: &mut impl Iterator<Item = TypePath>) -> ItemFn {
    // Check that function for special inputs such as 's: StringLike'. If found, replace with generics such as 's: S0' and remember.
    let (new_inputs, generic_params, stmts) = transform_inputs(&old_fn.sig.inputs, generic_gen);

    // For each special input found, define a new generic, for example, 'S0 : AsRef<str>'
    let new_generics = transform_generics(&old_fn.sig.generics.params, generic_params);

    // For each special input found, add a statement defining a new local variable. For example, 'let s = s.as_ref();'
    let new_stmts = transform_stmts(&old_fn.block.stmts, stmts);

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
    type Item = TypePath;

    fn next(&mut self) -> Option<Self::Item> {
        let s = format!("U{}_{}", self.uuid, self.counter);
        let result = parse_str(&s).expect("parse failure"); // cmk
        self.counter += 1;
        Some(result)
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
fn transform_inputs(
    old_inputs: &Punctuated<FnArg, Comma>,
    generic_gen: &mut impl Iterator<Item = TypePath>,
) -> (Punctuated<FnArg, Comma>, Vec<GenericParam>, Vec<Stmt>) {
    // For each old input, create a new input, transforming the type if it is special.
    let mut new_fn_args = Punctuated::<FnArg, Comma>::new();
    // Remember the names and types of the special inputs.
    let mut generic_params: Vec<GenericParam> = vec![];
    let mut stmts: Vec<Stmt> = vec![];

    for old_fn_arg in old_inputs {
        let delta_fn_arg = process_fn_arg(old_fn_arg, generic_gen);
        stmts = [stmts, delta_fn_arg.stmts].concat();
        generic_params = [generic_params, delta_fn_arg.generic_params].concat();
        new_fn_args.push(delta_fn_arg.fn_arg);
        // see https://doc.rust-lang.org/book/ch18-03-pattern-syntax.html#destructuring-nested-structs-and-enums
        // todo: Do these struct contains Box to make them easier to modify?
        // The box pattern syntax is experimental and can't use used in stable Rust.
    }
    (new_fn_args, generic_params, stmts)
}

#[derive(Debug)]
struct DeltaFnArg {
    fn_arg: FnArg,
    generic_params: Vec<GenericParam>,
    stmts: Vec<Stmt>,
}

fn process_fn_arg(
    old_fn_arg: &FnArg,
    generic_gen: &mut impl Iterator<Item = TypePath>,
) -> DeltaFnArg {
    // If the function input is normal (not self, not a macro, etc) ...
    if let Some((pat_ident, pat_type)) = is_normal_fn_arg(old_fn_arg) {
        let (delta_pat_type, new_pat_type) = replace_any_specials(pat_type.clone(), generic_gen);

        let new_fn_arg = FnArg::Typed(new_pat_type);

        DeltaFnArg {
            fn_arg: new_fn_arg,
            stmts: delta_pat_type.generate_any_stmts(pat_ident),
            generic_params: delta_pat_type.generic_params,
        }
    } else {
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
    generic_gen: &mut impl Iterator<Item = TypePath>,
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
    generic_gen: &'a mut dyn Iterator<Item = TypePath>,
    last_special: Option<Special>,
}

impl Fold for DeltaPatType<'_> {
    fn fold_type_path(&mut self, type_path: TypePath) -> TypePath {
        println!("fold_type_path (before): {:?}", quote!(#type_path));

        // Search for any special (sub)subtypes, replacing them with generics.
        let mut type_path = fold_type_path(self, type_path);

        // If this top-level type is special, replace it with a generic.
        if let Some((segment, special)) = is_special_type_path(&type_path) {
            self.last_special = Some(special.clone()); // remember which kind of special found

            type_path = self.generic_gen.next().unwrap(); // Generate the generic type, e.g. S23

            // Define the generic type, e.g. S23: AsRef<str>, and remember it.
            let sub_type = has_sub_type(segment.arguments); // Find anything inside angle brackets.
            let generic_param = special.special_to_generic_param(&type_path, sub_type.as_ref());
            self.generic_params.push(generic_param);
        } else {
            self.last_special = None;
        }
        println!("fold_type_path (after): {}", quote!(#type_path));
        type_path
    }
}

fn has_sub_type(args: PathArguments) -> Option<Type> {
    match args {
        PathArguments::None => None,
        PathArguments::AngleBracketed(ref args) => {
            let arg = first_and_only(args.args.iter()).expect("expected one argument cmk");
            println!("arg: {}", quote!(#arg));
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
    if let Some(segment) = first_and_only(type_path.path.segments.iter()) {
        let ident_string = segment.ident.to_string();
        if let Ok(special) = Special::from_str(&ident_string) {
            Some((segment.clone(), special))
        } else {
            None
        }
    } else {
        None
    }
}

// Define generics for each special input type. For example, 'S0 : AsRef<str>'
#[allow(clippy::ptr_arg)]
fn transform_generics(
    old_params: &Punctuated<GenericParam, Comma>,
    generic_params: Vec<GenericParam>,
) -> Punctuated<GenericParam, Comma> {
    let mut new_params = old_params.clone();
    new_params.extend(generic_params);
    new_params
}

// For each special input type, define a new local variable. For example, 'let s = s.as_ref();'
#[allow(clippy::ptr_arg)]
fn transform_stmts(old_stmts: &Vec<Stmt>, stmts: Vec<Stmt>) -> Vec<Stmt> {
    let mut new_stmts = old_stmts.clone();
    for (index, new_stmt) in stmts.iter().enumerate() {
        new_stmts.insert(index, new_stmt.clone()); // cmk too much cloning
    }
    new_stmts
}

#[cfg(test)]
mod tests {
    // cmk use prettyplease::unparse;
    use crate::{transform_fn, DeltaPatType, UuidGenerator};
    // cmk remove from cargo use prettyplease::unparse;
    use quote::quote;
    use syn::{fold::Fold, parse_quote, parse_str, ItemFn, TypePath};

    fn str_to_type_path(s: &str) -> TypePath {
        parse_str(s).unwrap()
    }

    fn generic_gen_test_factory() -> impl Iterator<Item = TypePath> + 'static {
        (0usize..)
            .into_iter()
            .map(|i| str_to_type_path(&format!("S{i}")))
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
            let a = a.as_ref();
            let b = b.as_ref();
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
}
