// todo rename to input-like-derive (or derive-input-like)
// todo remove cargo stuff features of syn no longer needed.
// todo use AST spans test so that problems with the user's syntax are reported correctly
//           see quote_spanned! in https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs
// todo add nice error enum

// cmk Look more at https://github.com/dtolnay/syn/tree/master/examples/trace-var
// cmk what about Vec<StringLike>?

use quote::quote;
use syn::__private::TokenStream; // todo don't use private
use syn::{
    parse_macro_input, parse_quote, parse_str, punctuated::Punctuated, token::Comma, Block, FnArg,
    GenericArgument, GenericParam, Generics, Ident, ItemFn, Pat, PatType, PathArguments, Signature,
    Stmt, Type, Type::Path,
};
use syn::{PatIdent, PathSegment};
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
    fn string_1(new_type: &Type, _sub_type: Option<&Type>) -> GenericParam {
        parse_quote!(#new_type : AsRef<str>)
    }
    fn string_2(name: Ident) -> Stmt {
        parse_quote! {
            let #name = #name.as_ref();
        }
    }
    fn path_1(new_type: &Type, _sub_type: Option<&Type>) -> GenericParam {
        parse_quote!(#new_type : AsRef<std::path::Path>)
    }
    fn path_2(name: Ident) -> Stmt {
        parse_quote! {
            let #name = #name.as_ref();
        }
    }

    fn iter_1(new_type: &Type, sub_type: Option<&Type>) -> GenericParam {
        let sub_type = sub_type.expect("iter_1: sub_type");
        parse_quote!(#new_type : IntoIterator<Item = #sub_type>)
    }
    fn iter_2(name: Ident) -> Stmt {
        parse_quote! {
            let #name = #name.into_iter();
        }
    }

    // cmk use Traits
    // cmk use a Hash table
    let likes = vec![
        Like {
            special: Ident::new("IterLike", proc_macro2::Span::call_site()),
            like_to_generic_param: &iter_1,
            ident_to_stmt: &iter_2,
        },
        Like {
            special: Ident::new("StringLike", proc_macro2::Span::call_site()),
            like_to_generic_param: &string_1,
            ident_to_stmt: &string_2,
        },
        Like {
            special: Ident::new("PathLike", proc_macro2::Span::call_site()),
            like_to_generic_param: &path_1,
            ident_to_stmt: &path_2,
        },
    ];

    // Check that function for special inputs such as 's: StringLike'. If found, replace with generics such as 's: S0' and remember.
    let (new_inputs, generic_params, stmts) =
        transform_inputs(&old_fn.sig.inputs, generic_gen, likes);

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
    type Item = Type;

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

// Look for special inputs such as 's: StringLike'. If found, replace with generics like 's: S0'.
// Todo support: PathLike, IterLike<T>, ArrayLike<T> (including ArrayLike<PathLike>), NdArrayLike<T>, etc.

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
    generic_gen: &mut impl Iterator<Item = Type>,
    likes: Vec<Like>,
) -> (Punctuated<FnArg, Comma>, Vec<GenericParam>, Vec<Stmt>) {
    // For each old input, create a new input, transforming the type if it is special.
    let mut new_fn_args = Punctuated::<FnArg, Comma>::new();
    // Remember the names and types of the special inputs.
    let mut generic_params: Vec<GenericParam> = vec![];
    let mut stmts: Vec<Stmt> = vec![];

    for old_fn_arg in old_inputs {
        let delta_fn_arg = process_fn_arg(old_fn_arg, &likes, generic_gen);
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
    likes: &Vec<Like>,
    generic_gen: &mut impl Iterator<Item = Type>,
) -> DeltaFnArg {
    // If the input is 'Typed' (so not self), and
    // the 'pat' (aka variable) field is variant 'Ident' (so not, for example, a macro), and
    // the type is 'Path' (so not, for example, a macro)
    if let Some((pat_ident, pat_type)) = is_normal_fn_arg(old_fn_arg) {
        // the one and only item in path is, for example, 'StringLike'
        let delta_type = process_type(&*pat_type.ty, likes, generic_gen);

        let new_fn_arg = FnArg::Typed(PatType {
            ty: Box::new(delta_type.new_type.clone()),
            ..pat_type.clone()
        });
        let stmts = generate_any_stmts(&delta_type, pat_ident);
        DeltaFnArg {
            fn_arg: new_fn_arg,
            generic_params: delta_type.generic_params,
            stmts,
        }
    } else {
        DeltaFnArg {
            fn_arg: old_fn_arg.clone(),
            generic_params: vec![],
            stmts: vec![],
        }
    }
}

fn generate_any_stmts(delta_type: &DeltaType, pat_ident: &PatIdent) -> Vec<Stmt> {
    if let Some(like) = &delta_type.like {
        let name = pat_ident.ident.clone(); // cmk too many clones
        vec![(like.ident_to_stmt)(name)]
    } else {
        vec![]
    }
}

fn is_normal_fn_arg(arg: &FnArg) -> Option<(&PatIdent, &PatType)> {
    if let FnArg::Typed(pat_type) = arg {
        if let Pat::Ident(pat_ident) = &*pat_type.pat {
            if let Type::Path(_) = &*pat_type.ty {
                return Some((pat_ident, pat_type));
            }
        }
    }
    None
}

struct DeltaType {
    new_type: Type,
    like: Option<Like>,
    generic_params: Vec<GenericParam>,
}

// fn process_type_new(
//     ty: &Type,
//     likes: &Vec<Like>,
//     generic_gen: &mut impl Iterator<Item = Type>,
// ) -> DeltaType {
//     // Search type and its subtypes for special types starting at the deepest level.
//     // When one is found, replace it with a generic.
//     // Finally, return the new type, a list of the generics. Also, if the top-level type was special, return the special type.
// }

fn process_type(
    ty: &Type,
    likes: &Vec<Like>,
    generic_gen: &mut impl Iterator<Item = Type>,
) -> DeltaType {
    if let Some((segment, like)) = is_special_type(ty, likes) {
        // v: StringLike -> v: S0, <S0: AsRef<str>>, {let v = v.as_ref();}
        // v: IterLike<i32> -> v: S0, <S0: IntoIterator<Item = i32>>, {let v = v.into_iter();}
        // v: IterLike<StringLike> -> v: S0, <S0: IntoIterator<Item = S1>, S1: AsRef<str>>, {let v = v.into_iter();}
        // v: IterLike<IterLike<i32>> -> v: S0, <S0: IntoIterator<Item = S1>, S1: IntoIterator<Item = i32>>, {let v = v.into_iter();}
        // v: IterLike<IterLike<StringLike>> -> v: S0, <S0: IntoIterator<Item = S1>, S1: IntoIterator<Item = S2>, S2: AsRef<str>>, {let v = v.into_iter();}

        // If Like<something> is found,
        //      process something, returning the perhaps new subtype (any maybe new generics),
        let (sub_type, sub_generic_params) = process_any_subtype(segment, likes, generic_gen);

        // define our own generic type -- for example S0 -- and add it to the list of generics
        let new_type = generic_gen.next().unwrap();
        // cmk why does the like_to_generic_param function need a move input?
        let generic_param = (like.like_to_generic_param)(&new_type, sub_type.as_ref());

        DeltaType {
            like: Some(like),
            new_type,
            generic_params: [sub_generic_params, vec![generic_param]].concat(),
        }
    } else {
        DeltaType {
            like: None,
            new_type: ty.clone(),
            generic_params: vec![],
        }
    }
}

fn process_any_subtype(
    segment: PathSegment,
    likes: &Vec<Like>,
    generic_gen: &mut impl Iterator<Item = Type>,
) -> (Option<Type>, Vec<GenericParam>) {
    if let Some(sub_type_inner) = has_sub_type(segment.arguments) {
        let sub_delta2 = process_type(&sub_type_inner, likes, generic_gen);
        (Some(sub_delta2.new_type), sub_delta2.generic_params)
    } else {
        (None, vec![])
    }
}

fn has_sub_type(args: PathArguments) -> Option<Type> {
    match args {
        PathArguments::None => None,
        PathArguments::AngleBracketed(ref args) => {
            let arg = first_and_only(args.args.iter()).expect("expected one argument cmk");
            print!("arg: {:#?}", arg);
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

fn is_special_type(ty: &Type, likes: &Vec<Like>) -> Option<(PathSegment, Like)> {
    if let Path(type_path) = ty {
        // print!("type_path: {:#?}", type_path);
        if let Some(segment) = first_and_only(type_path.path.segments.iter()) {
            // print!("segment: {:#?}", segment);
            for like in likes {
                // print!("{:#?}=={:#?} ", segment.ident, like.special);
                if segment.ident == like.special {
                    // Create a new input with a generic type and remember the name and type.
                    return Some((segment.clone(), like.clone())); // todo review all clones
                }
            }
        }
    }
    None
}

// Define generics for each special input type. For example, 'S0 : AsRef<str>'
#[allow(clippy::ptr_arg)]
fn transform_generics(
    old_params: &Punctuated<GenericParam, Comma>,
    generic_params: Vec<GenericParam>,
) -> Punctuated<GenericParam, Comma> {
    let mut new_params = old_params.clone();
    for new_param in generic_params.iter() {
        new_params.push(new_param.clone()); // cmk too much cloning
    }
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

#[derive(Clone)]
struct Like {
    special: Ident,
    like_to_generic_param: &'static dyn Fn(&Type, Option<&Type>) -> GenericParam,
    ident_to_stmt: &'static dyn Fn(Ident) -> Stmt,
}

#[cfg(test)]
mod tests {
    // cmk use prettyplease::unparse;
    use crate::{transform_fn, UuidGenerator};
    use prettyplease::unparse;
    use syn::{parse_quote, parse_str, File, Item, ItemFn, Type};

    fn str_to_type(s: &str) -> Type {
        parse_str(s).unwrap()
    }

    fn item_fn_to_string(item_fn: &ItemFn) -> String {
        let old_file = parse_str::<File>("").expect("doesn't parse"); // todo is there a File::new?
        let new_file = File {
            items: vec![Item::Fn(item_fn.clone())],
            ..old_file
        };
        unparse(&new_file)
    }

    fn generic_gen_test_factory() -> impl Iterator<Item = Type> + 'static {
        (0usize..)
            .into_iter()
            .map(|i| str_to_type(&format!("S{i}")))
    }

    fn assert_item_fn_eq(after: &ItemFn, expected: &ItemFn) {
        if after == expected {
            return;
        }

        let after_str = item_fn_to_string(after);
        let expected_str = item_fn_to_string(expected);
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
            i: S1,
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
            i: Vec<S0>,
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
}
