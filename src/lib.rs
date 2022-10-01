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
use ndarray;
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
            Special::NdArrayLike => {
                let sub_type = sub_type.expect("nd_array: sub_type");
                parse_quote!(#new_type : A: Into<ndarray::ArrayView1<#sub_type>>)
                // should be <'a, T: 'a, A: Into<nd::ArrayView1<'a, T>>
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
        println!("fold_type_path (before): {:?}", quote!(#type_path));

        // Search for any special (sub)subtypes, replacing them with generics.
        let mut type_path = fold_type_path(self, type_path);

        // If this top-level type is special, replace it with a generic.
        if let Some((segment, special)) = is_special_type_path(&type_path) {
            self.last_special = Some(special.clone()); // remember which kind of special found

            let s = self.generic_gen.next().unwrap(); // Generate the generic type, e.g. S23
            type_path = parse_str(&s).expect("parse failure");

            // Define the generic type, e.g. S23: AsRef<str>, and remember it.
            let sub_type = has_sub_type(segment.arguments); // Find anything inside angle brackets.
            let generic_param = special.special_to_generic_param(&type_path, sub_type.as_ref());
            self.generic_params.push(generic_param);

            if special.should_add_lifetime() {
                let s = self.generic_gen.next().unwrap(); // Generate the generic type, e.g. S23
                let lifetime: Lifetime = parse_str(&s).expect("parse failure"); // cmk 9 rules: best & easy way to create an object?
                let generic_param: GenericParam = parse_quote! { #lifetime };
                self.generic_params.push(generic_param);

                type_path = parse_quote! { #type_path<#lifetime> };
                //cmk GenericParam::Lifetime(LifetimeDef::new(lifetime)); // cmk 9 rules: This is another way to create an object.
            }
        } else {
            self.last_special = None;
        }
        println!("fold_type_path (after): {}", quote!(#type_path));
        type_path
    }

    fn fold_abi(&mut self, i: syn::Abi) -> syn::Abi {
        syn::fold::fold_abi(self, i)
    }

    fn fold_angle_bracketed_generic_arguments(
        &mut self,
        i: syn::AngleBracketedGenericArguments,
    ) -> syn::AngleBracketedGenericArguments {
        syn::fold::fold_angle_bracketed_generic_arguments(self, i)
    }

    fn fold_arm(&mut self, i: syn::Arm) -> syn::Arm {
        syn::fold::fold_arm(self, i)
    }

    fn fold_attr_style(&mut self, i: syn::AttrStyle) -> syn::AttrStyle {
        syn::fold::fold_attr_style(self, i)
    }

    fn fold_attribute(&mut self, i: syn::Attribute) -> syn::Attribute {
        syn::fold::fold_attribute(self, i)
    }

    fn fold_bare_fn_arg(&mut self, i: syn::BareFnArg) -> syn::BareFnArg {
        syn::fold::fold_bare_fn_arg(self, i)
    }

    fn fold_bin_op(&mut self, i: syn::BinOp) -> syn::BinOp {
        syn::fold::fold_bin_op(self, i)
    }

    fn fold_binding(&mut self, i: syn::Binding) -> syn::Binding {
        syn::fold::fold_binding(self, i)
    }

    fn fold_block(&mut self, i: Block) -> Block {
        syn::fold::fold_block(self, i)
    }

    fn fold_bound_lifetimes(&mut self, i: syn::BoundLifetimes) -> syn::BoundLifetimes {
        syn::fold::fold_bound_lifetimes(self, i)
    }

    fn fold_const_param(&mut self, i: syn::ConstParam) -> syn::ConstParam {
        syn::fold::fold_const_param(self, i)
    }

    fn fold_constraint(&mut self, i: syn::Constraint) -> syn::Constraint {
        syn::fold::fold_constraint(self, i)
    }

    fn fold_data(&mut self, i: syn::Data) -> syn::Data {
        syn::fold::fold_data(self, i)
    }

    fn fold_data_enum(&mut self, i: syn::DataEnum) -> syn::DataEnum {
        syn::fold::fold_data_enum(self, i)
    }

    fn fold_data_struct(&mut self, i: syn::DataStruct) -> syn::DataStruct {
        syn::fold::fold_data_struct(self, i)
    }

    fn fold_data_union(&mut self, i: syn::DataUnion) -> syn::DataUnion {
        syn::fold::fold_data_union(self, i)
    }

    fn fold_derive_input(&mut self, i: syn::DeriveInput) -> syn::DeriveInput {
        syn::fold::fold_derive_input(self, i)
    }

    fn fold_expr(&mut self, i: syn::Expr) -> syn::Expr {
        syn::fold::fold_expr(self, i)
    }

    fn fold_expr_array(&mut self, i: syn::ExprArray) -> syn::ExprArray {
        syn::fold::fold_expr_array(self, i)
    }

    fn fold_expr_assign(&mut self, i: syn::ExprAssign) -> syn::ExprAssign {
        syn::fold::fold_expr_assign(self, i)
    }

    fn fold_expr_assign_op(&mut self, i: syn::ExprAssignOp) -> syn::ExprAssignOp {
        syn::fold::fold_expr_assign_op(self, i)
    }

    fn fold_expr_async(&mut self, i: syn::ExprAsync) -> syn::ExprAsync {
        syn::fold::fold_expr_async(self, i)
    }

    fn fold_expr_await(&mut self, i: syn::ExprAwait) -> syn::ExprAwait {
        syn::fold::fold_expr_await(self, i)
    }

    fn fold_expr_binary(&mut self, i: syn::ExprBinary) -> syn::ExprBinary {
        syn::fold::fold_expr_binary(self, i)
    }

    fn fold_expr_block(&mut self, i: syn::ExprBlock) -> syn::ExprBlock {
        syn::fold::fold_expr_block(self, i)
    }

    fn fold_expr_box(&mut self, i: syn::ExprBox) -> syn::ExprBox {
        syn::fold::fold_expr_box(self, i)
    }

    fn fold_expr_break(&mut self, i: syn::ExprBreak) -> syn::ExprBreak {
        syn::fold::fold_expr_break(self, i)
    }

    fn fold_expr_call(&mut self, i: syn::ExprCall) -> syn::ExprCall {
        syn::fold::fold_expr_call(self, i)
    }

    fn fold_expr_cast(&mut self, i: syn::ExprCast) -> syn::ExprCast {
        syn::fold::fold_expr_cast(self, i)
    }

    fn fold_expr_closure(&mut self, i: syn::ExprClosure) -> syn::ExprClosure {
        syn::fold::fold_expr_closure(self, i)
    }

    fn fold_expr_continue(&mut self, i: syn::ExprContinue) -> syn::ExprContinue {
        syn::fold::fold_expr_continue(self, i)
    }

    fn fold_expr_field(&mut self, i: syn::ExprField) -> syn::ExprField {
        syn::fold::fold_expr_field(self, i)
    }

    fn fold_expr_for_loop(&mut self, i: syn::ExprForLoop) -> syn::ExprForLoop {
        syn::fold::fold_expr_for_loop(self, i)
    }

    fn fold_expr_group(&mut self, i: syn::ExprGroup) -> syn::ExprGroup {
        syn::fold::fold_expr_group(self, i)
    }

    fn fold_expr_if(&mut self, i: syn::ExprIf) -> syn::ExprIf {
        syn::fold::fold_expr_if(self, i)
    }

    fn fold_expr_index(&mut self, i: syn::ExprIndex) -> syn::ExprIndex {
        syn::fold::fold_expr_index(self, i)
    }

    fn fold_expr_let(&mut self, i: syn::ExprLet) -> syn::ExprLet {
        syn::fold::fold_expr_let(self, i)
    }

    fn fold_expr_lit(&mut self, i: syn::ExprLit) -> syn::ExprLit {
        syn::fold::fold_expr_lit(self, i)
    }

    fn fold_expr_loop(&mut self, i: syn::ExprLoop) -> syn::ExprLoop {
        syn::fold::fold_expr_loop(self, i)
    }

    fn fold_expr_macro(&mut self, i: syn::ExprMacro) -> syn::ExprMacro {
        syn::fold::fold_expr_macro(self, i)
    }

    fn fold_expr_match(&mut self, i: syn::ExprMatch) -> syn::ExprMatch {
        syn::fold::fold_expr_match(self, i)
    }

    fn fold_expr_method_call(&mut self, i: syn::ExprMethodCall) -> syn::ExprMethodCall {
        syn::fold::fold_expr_method_call(self, i)
    }

    fn fold_expr_paren(&mut self, i: syn::ExprParen) -> syn::ExprParen {
        syn::fold::fold_expr_paren(self, i)
    }

    fn fold_expr_path(&mut self, i: syn::ExprPath) -> syn::ExprPath {
        syn::fold::fold_expr_path(self, i)
    }

    fn fold_expr_range(&mut self, i: syn::ExprRange) -> syn::ExprRange {
        syn::fold::fold_expr_range(self, i)
    }

    fn fold_expr_reference(&mut self, i: syn::ExprReference) -> syn::ExprReference {
        syn::fold::fold_expr_reference(self, i)
    }

    fn fold_expr_repeat(&mut self, i: syn::ExprRepeat) -> syn::ExprRepeat {
        syn::fold::fold_expr_repeat(self, i)
    }

    fn fold_expr_return(&mut self, i: syn::ExprReturn) -> syn::ExprReturn {
        syn::fold::fold_expr_return(self, i)
    }

    fn fold_expr_struct(&mut self, i: syn::ExprStruct) -> syn::ExprStruct {
        syn::fold::fold_expr_struct(self, i)
    }

    fn fold_expr_try(&mut self, i: syn::ExprTry) -> syn::ExprTry {
        syn::fold::fold_expr_try(self, i)
    }

    fn fold_expr_try_block(&mut self, i: syn::ExprTryBlock) -> syn::ExprTryBlock {
        syn::fold::fold_expr_try_block(self, i)
    }

    fn fold_expr_tuple(&mut self, i: syn::ExprTuple) -> syn::ExprTuple {
        syn::fold::fold_expr_tuple(self, i)
    }

    fn fold_expr_type(&mut self, i: syn::ExprType) -> syn::ExprType {
        syn::fold::fold_expr_type(self, i)
    }

    fn fold_expr_unary(&mut self, i: syn::ExprUnary) -> syn::ExprUnary {
        syn::fold::fold_expr_unary(self, i)
    }

    fn fold_expr_unsafe(&mut self, i: syn::ExprUnsafe) -> syn::ExprUnsafe {
        syn::fold::fold_expr_unsafe(self, i)
    }

    fn fold_expr_while(&mut self, i: syn::ExprWhile) -> syn::ExprWhile {
        syn::fold::fold_expr_while(self, i)
    }

    fn fold_expr_yield(&mut self, i: syn::ExprYield) -> syn::ExprYield {
        syn::fold::fold_expr_yield(self, i)
    }

    fn fold_field(&mut self, i: syn::Field) -> syn::Field {
        syn::fold::fold_field(self, i)
    }

    fn fold_field_pat(&mut self, i: syn::FieldPat) -> syn::FieldPat {
        syn::fold::fold_field_pat(self, i)
    }

    fn fold_field_value(&mut self, i: syn::FieldValue) -> syn::FieldValue {
        syn::fold::fold_field_value(self, i)
    }

    fn fold_fields(&mut self, i: syn::Fields) -> syn::Fields {
        syn::fold::fold_fields(self, i)
    }

    fn fold_fields_named(&mut self, i: syn::FieldsNamed) -> syn::FieldsNamed {
        syn::fold::fold_fields_named(self, i)
    }

    fn fold_fields_unnamed(&mut self, i: syn::FieldsUnnamed) -> syn::FieldsUnnamed {
        syn::fold::fold_fields_unnamed(self, i)
    }

    fn fold_file(&mut self, i: syn::File) -> syn::File {
        syn::fold::fold_file(self, i)
    }

    fn fold_fn_arg(&mut self, i: FnArg) -> FnArg {
        syn::fold::fold_fn_arg(self, i)
    }

    fn fold_foreign_item(&mut self, i: syn::ForeignItem) -> syn::ForeignItem {
        syn::fold::fold_foreign_item(self, i)
    }

    fn fold_foreign_item_fn(&mut self, i: syn::ForeignItemFn) -> syn::ForeignItemFn {
        syn::fold::fold_foreign_item_fn(self, i)
    }

    fn fold_foreign_item_macro(&mut self, i: syn::ForeignItemMacro) -> syn::ForeignItemMacro {
        syn::fold::fold_foreign_item_macro(self, i)
    }

    fn fold_foreign_item_static(&mut self, i: syn::ForeignItemStatic) -> syn::ForeignItemStatic {
        syn::fold::fold_foreign_item_static(self, i)
    }

    fn fold_foreign_item_type(&mut self, i: syn::ForeignItemType) -> syn::ForeignItemType {
        syn::fold::fold_foreign_item_type(self, i)
    }

    fn fold_generic_argument(&mut self, i: GenericArgument) -> GenericArgument {
        syn::fold::fold_generic_argument(self, i)
    }

    fn fold_generic_method_argument(
        &mut self,
        i: syn::GenericMethodArgument,
    ) -> syn::GenericMethodArgument {
        syn::fold::fold_generic_method_argument(self, i)
    }

    fn fold_generic_param(&mut self, i: GenericParam) -> GenericParam {
        syn::fold::fold_generic_param(self, i)
    }

    fn fold_generics(&mut self, i: Generics) -> Generics {
        syn::fold::fold_generics(self, i)
    }

    fn fold_ident(&mut self, i: proc_macro2::Ident) -> proc_macro2::Ident {
        syn::fold::fold_ident(self, i)
    }

    fn fold_impl_item(&mut self, i: syn::ImplItem) -> syn::ImplItem {
        syn::fold::fold_impl_item(self, i)
    }

    fn fold_impl_item_const(&mut self, i: syn::ImplItemConst) -> syn::ImplItemConst {
        syn::fold::fold_impl_item_const(self, i)
    }

    fn fold_impl_item_macro(&mut self, i: syn::ImplItemMacro) -> syn::ImplItemMacro {
        syn::fold::fold_impl_item_macro(self, i)
    }

    fn fold_impl_item_method(&mut self, i: syn::ImplItemMethod) -> syn::ImplItemMethod {
        syn::fold::fold_impl_item_method(self, i)
    }

    fn fold_impl_item_type(&mut self, i: syn::ImplItemType) -> syn::ImplItemType {
        syn::fold::fold_impl_item_type(self, i)
    }

    fn fold_index(&mut self, i: syn::Index) -> syn::Index {
        syn::fold::fold_index(self, i)
    }

    fn fold_item(&mut self, i: syn::Item) -> syn::Item {
        syn::fold::fold_item(self, i)
    }

    fn fold_item_const(&mut self, i: syn::ItemConst) -> syn::ItemConst {
        syn::fold::fold_item_const(self, i)
    }

    fn fold_item_enum(&mut self, i: syn::ItemEnum) -> syn::ItemEnum {
        syn::fold::fold_item_enum(self, i)
    }

    fn fold_item_extern_crate(&mut self, i: syn::ItemExternCrate) -> syn::ItemExternCrate {
        syn::fold::fold_item_extern_crate(self, i)
    }

    fn fold_item_fn(&mut self, i: ItemFn) -> ItemFn {
        syn::fold::fold_item_fn(self, i)
    }

    fn fold_item_foreign_mod(&mut self, i: syn::ItemForeignMod) -> syn::ItemForeignMod {
        syn::fold::fold_item_foreign_mod(self, i)
    }

    fn fold_item_impl(&mut self, i: syn::ItemImpl) -> syn::ItemImpl {
        syn::fold::fold_item_impl(self, i)
    }

    fn fold_item_macro(&mut self, i: syn::ItemMacro) -> syn::ItemMacro {
        syn::fold::fold_item_macro(self, i)
    }

    fn fold_item_macro2(&mut self, i: syn::ItemMacro2) -> syn::ItemMacro2 {
        syn::fold::fold_item_macro2(self, i)
    }

    fn fold_item_mod(&mut self, i: syn::ItemMod) -> syn::ItemMod {
        syn::fold::fold_item_mod(self, i)
    }

    fn fold_item_static(&mut self, i: syn::ItemStatic) -> syn::ItemStatic {
        syn::fold::fold_item_static(self, i)
    }

    fn fold_item_struct(&mut self, i: syn::ItemStruct) -> syn::ItemStruct {
        syn::fold::fold_item_struct(self, i)
    }

    fn fold_item_trait(&mut self, i: syn::ItemTrait) -> syn::ItemTrait {
        syn::fold::fold_item_trait(self, i)
    }

    fn fold_item_trait_alias(&mut self, i: syn::ItemTraitAlias) -> syn::ItemTraitAlias {
        syn::fold::fold_item_trait_alias(self, i)
    }

    fn fold_item_type(&mut self, i: syn::ItemType) -> syn::ItemType {
        syn::fold::fold_item_type(self, i)
    }

    fn fold_item_union(&mut self, i: syn::ItemUnion) -> syn::ItemUnion {
        syn::fold::fold_item_union(self, i)
    }

    fn fold_item_use(&mut self, i: syn::ItemUse) -> syn::ItemUse {
        syn::fold::fold_item_use(self, i)
    }

    fn fold_label(&mut self, i: syn::Label) -> syn::Label {
        syn::fold::fold_label(self, i)
    }

    fn fold_lifetime(&mut self, i: syn::Lifetime) -> syn::Lifetime {
        syn::fold::fold_lifetime(self, i)
    }

    fn fold_lifetime_def(&mut self, i: syn::LifetimeDef) -> syn::LifetimeDef {
        syn::fold::fold_lifetime_def(self, i)
    }

    fn fold_lit(&mut self, i: syn::Lit) -> syn::Lit {
        syn::fold::fold_lit(self, i)
    }

    fn fold_lit_bool(&mut self, i: syn::LitBool) -> syn::LitBool {
        syn::fold::fold_lit_bool(self, i)
    }

    fn fold_lit_byte(&mut self, i: syn::LitByte) -> syn::LitByte {
        syn::fold::fold_lit_byte(self, i)
    }

    fn fold_lit_byte_str(&mut self, i: syn::LitByteStr) -> syn::LitByteStr {
        syn::fold::fold_lit_byte_str(self, i)
    }

    fn fold_lit_char(&mut self, i: syn::LitChar) -> syn::LitChar {
        syn::fold::fold_lit_char(self, i)
    }

    fn fold_lit_float(&mut self, i: syn::LitFloat) -> syn::LitFloat {
        syn::fold::fold_lit_float(self, i)
    }

    fn fold_lit_int(&mut self, i: syn::LitInt) -> syn::LitInt {
        syn::fold::fold_lit_int(self, i)
    }

    fn fold_lit_str(&mut self, i: syn::LitStr) -> syn::LitStr {
        syn::fold::fold_lit_str(self, i)
    }

    fn fold_local(&mut self, i: syn::Local) -> syn::Local {
        syn::fold::fold_local(self, i)
    }

    fn fold_macro(&mut self, i: syn::Macro) -> syn::Macro {
        syn::fold::fold_macro(self, i)
    }

    fn fold_macro_delimiter(&mut self, i: syn::MacroDelimiter) -> syn::MacroDelimiter {
        syn::fold::fold_macro_delimiter(self, i)
    }

    fn fold_member(&mut self, i: syn::Member) -> syn::Member {
        syn::fold::fold_member(self, i)
    }

    fn fold_meta(&mut self, i: syn::Meta) -> syn::Meta {
        syn::fold::fold_meta(self, i)
    }

    fn fold_meta_list(&mut self, i: syn::MetaList) -> syn::MetaList {
        syn::fold::fold_meta_list(self, i)
    }

    fn fold_meta_name_value(&mut self, i: syn::MetaNameValue) -> syn::MetaNameValue {
        syn::fold::fold_meta_name_value(self, i)
    }

    fn fold_method_turbofish(&mut self, i: syn::MethodTurbofish) -> syn::MethodTurbofish {
        syn::fold::fold_method_turbofish(self, i)
    }

    fn fold_nested_meta(&mut self, i: syn::NestedMeta) -> syn::NestedMeta {
        syn::fold::fold_nested_meta(self, i)
    }

    fn fold_parenthesized_generic_arguments(
        &mut self,
        i: syn::ParenthesizedGenericArguments,
    ) -> syn::ParenthesizedGenericArguments {
        syn::fold::fold_parenthesized_generic_arguments(self, i)
    }

    fn fold_pat(&mut self, i: Pat) -> Pat {
        syn::fold::fold_pat(self, i)
    }

    fn fold_pat_box(&mut self, i: syn::PatBox) -> syn::PatBox {
        syn::fold::fold_pat_box(self, i)
    }

    fn fold_pat_ident(&mut self, i: PatIdent) -> PatIdent {
        syn::fold::fold_pat_ident(self, i)
    }

    fn fold_pat_lit(&mut self, i: syn::PatLit) -> syn::PatLit {
        syn::fold::fold_pat_lit(self, i)
    }

    fn fold_pat_macro(&mut self, i: syn::PatMacro) -> syn::PatMacro {
        syn::fold::fold_pat_macro(self, i)
    }

    fn fold_pat_or(&mut self, i: syn::PatOr) -> syn::PatOr {
        syn::fold::fold_pat_or(self, i)
    }

    fn fold_pat_path(&mut self, i: syn::PatPath) -> syn::PatPath {
        syn::fold::fold_pat_path(self, i)
    }

    fn fold_pat_range(&mut self, i: syn::PatRange) -> syn::PatRange {
        syn::fold::fold_pat_range(self, i)
    }

    fn fold_pat_reference(&mut self, i: syn::PatReference) -> syn::PatReference {
        syn::fold::fold_pat_reference(self, i)
    }

    fn fold_pat_rest(&mut self, i: syn::PatRest) -> syn::PatRest {
        syn::fold::fold_pat_rest(self, i)
    }

    fn fold_pat_slice(&mut self, i: syn::PatSlice) -> syn::PatSlice {
        syn::fold::fold_pat_slice(self, i)
    }

    fn fold_pat_struct(&mut self, i: syn::PatStruct) -> syn::PatStruct {
        syn::fold::fold_pat_struct(self, i)
    }

    fn fold_pat_tuple(&mut self, i: syn::PatTuple) -> syn::PatTuple {
        syn::fold::fold_pat_tuple(self, i)
    }

    fn fold_pat_tuple_struct(&mut self, i: syn::PatTupleStruct) -> syn::PatTupleStruct {
        syn::fold::fold_pat_tuple_struct(self, i)
    }

    fn fold_pat_type(&mut self, i: PatType) -> PatType {
        syn::fold::fold_pat_type(self, i)
    }

    fn fold_pat_wild(&mut self, i: syn::PatWild) -> syn::PatWild {
        syn::fold::fold_pat_wild(self, i)
    }

    fn fold_path(&mut self, i: syn::Path) -> syn::Path {
        syn::fold::fold_path(self, i)
    }

    fn fold_path_arguments(&mut self, i: PathArguments) -> PathArguments {
        syn::fold::fold_path_arguments(self, i)
    }

    fn fold_path_segment(&mut self, i: PathSegment) -> PathSegment {
        syn::fold::fold_path_segment(self, i)
    }

    fn fold_predicate_eq(&mut self, i: syn::PredicateEq) -> syn::PredicateEq {
        syn::fold::fold_predicate_eq(self, i)
    }

    fn fold_predicate_lifetime(&mut self, i: syn::PredicateLifetime) -> syn::PredicateLifetime {
        syn::fold::fold_predicate_lifetime(self, i)
    }

    fn fold_predicate_type(&mut self, i: syn::PredicateType) -> syn::PredicateType {
        syn::fold::fold_predicate_type(self, i)
    }

    fn fold_qself(&mut self, i: syn::QSelf) -> syn::QSelf {
        syn::fold::fold_qself(self, i)
    }

    fn fold_range_limits(&mut self, i: syn::RangeLimits) -> syn::RangeLimits {
        syn::fold::fold_range_limits(self, i)
    }

    fn fold_receiver(&mut self, i: syn::Receiver) -> syn::Receiver {
        syn::fold::fold_receiver(self, i)
    }

    fn fold_return_type(&mut self, i: syn::ReturnType) -> syn::ReturnType {
        syn::fold::fold_return_type(self, i)
    }

    fn fold_signature(&mut self, i: Signature) -> Signature {
        syn::fold::fold_signature(self, i)
    }

    fn fold_span(&mut self, i: proc_macro2::Span) -> proc_macro2::Span {
        syn::fold::fold_span(self, i)
    }

    fn fold_stmt(&mut self, i: Stmt) -> Stmt {
        syn::fold::fold_stmt(self, i)
    }

    fn fold_trait_bound(&mut self, i: syn::TraitBound) -> syn::TraitBound {
        syn::fold::fold_trait_bound(self, i)
    }

    fn fold_trait_bound_modifier(&mut self, i: syn::TraitBoundModifier) -> syn::TraitBoundModifier {
        syn::fold::fold_trait_bound_modifier(self, i)
    }

    fn fold_trait_item(&mut self, i: syn::TraitItem) -> syn::TraitItem {
        syn::fold::fold_trait_item(self, i)
    }

    fn fold_trait_item_const(&mut self, i: syn::TraitItemConst) -> syn::TraitItemConst {
        syn::fold::fold_trait_item_const(self, i)
    }

    fn fold_trait_item_macro(&mut self, i: syn::TraitItemMacro) -> syn::TraitItemMacro {
        syn::fold::fold_trait_item_macro(self, i)
    }

    fn fold_trait_item_method(&mut self, i: syn::TraitItemMethod) -> syn::TraitItemMethod {
        syn::fold::fold_trait_item_method(self, i)
    }

    fn fold_trait_item_type(&mut self, i: syn::TraitItemType) -> syn::TraitItemType {
        syn::fold::fold_trait_item_type(self, i)
    }

    fn fold_type(&mut self, i: Type) -> Type {
        syn::fold::fold_type(self, i)
    }

    fn fold_type_array(&mut self, i: syn::TypeArray) -> syn::TypeArray {
        syn::fold::fold_type_array(self, i)
    }

    fn fold_type_bare_fn(&mut self, i: syn::TypeBareFn) -> syn::TypeBareFn {
        syn::fold::fold_type_bare_fn(self, i)
    }

    fn fold_type_group(&mut self, i: syn::TypeGroup) -> syn::TypeGroup {
        syn::fold::fold_type_group(self, i)
    }

    fn fold_type_impl_trait(&mut self, i: syn::TypeImplTrait) -> syn::TypeImplTrait {
        syn::fold::fold_type_impl_trait(self, i)
    }

    fn fold_type_infer(&mut self, i: syn::TypeInfer) -> syn::TypeInfer {
        syn::fold::fold_type_infer(self, i)
    }

    fn fold_type_macro(&mut self, i: syn::TypeMacro) -> syn::TypeMacro {
        syn::fold::fold_type_macro(self, i)
    }

    fn fold_type_never(&mut self, i: syn::TypeNever) -> syn::TypeNever {
        syn::fold::fold_type_never(self, i)
    }

    fn fold_type_param(&mut self, i: syn::TypeParam) -> syn::TypeParam {
        syn::fold::fold_type_param(self, i)
    }

    fn fold_type_param_bound(&mut self, i: syn::TypeParamBound) -> syn::TypeParamBound {
        syn::fold::fold_type_param_bound(self, i)
    }

    fn fold_type_paren(&mut self, i: syn::TypeParen) -> syn::TypeParen {
        syn::fold::fold_type_paren(self, i)
    }

    fn fold_type_ptr(&mut self, i: syn::TypePtr) -> syn::TypePtr {
        syn::fold::fold_type_ptr(self, i)
    }

    fn fold_type_reference(&mut self, i: syn::TypeReference) -> syn::TypeReference {
        syn::fold::fold_type_reference(self, i)
    }

    fn fold_type_slice(&mut self, i: syn::TypeSlice) -> syn::TypeSlice {
        syn::fold::fold_type_slice(self, i)
    }

    fn fold_type_trait_object(&mut self, i: syn::TypeTraitObject) -> syn::TypeTraitObject {
        syn::fold::fold_type_trait_object(self, i)
    }

    fn fold_type_tuple(&mut self, i: syn::TypeTuple) -> syn::TypeTuple {
        syn::fold::fold_type_tuple(self, i)
    }

    fn fold_un_op(&mut self, i: syn::UnOp) -> syn::UnOp {
        syn::fold::fold_un_op(self, i)
    }

    fn fold_use_glob(&mut self, i: syn::UseGlob) -> syn::UseGlob {
        syn::fold::fold_use_glob(self, i)
    }

    fn fold_use_group(&mut self, i: syn::UseGroup) -> syn::UseGroup {
        syn::fold::fold_use_group(self, i)
    }

    fn fold_use_name(&mut self, i: syn::UseName) -> syn::UseName {
        syn::fold::fold_use_name(self, i)
    }

    fn fold_use_path(&mut self, i: syn::UsePath) -> syn::UsePath {
        syn::fold::fold_use_path(self, i)
    }

    fn fold_use_rename(&mut self, i: syn::UseRename) -> syn::UseRename {
        syn::fold::fold_use_rename(self, i)
    }

    fn fold_use_tree(&mut self, i: syn::UseTree) -> syn::UseTree {
        syn::fold::fold_use_tree(self, i)
    }

    fn fold_variadic(&mut self, i: syn::Variadic) -> syn::Variadic {
        syn::fold::fold_variadic(self, i)
    }

    fn fold_variant(&mut self, i: syn::Variant) -> syn::Variant {
        syn::fold::fold_variant(self, i)
    }

    fn fold_vis_crate(&mut self, i: syn::VisCrate) -> syn::VisCrate {
        syn::fold::fold_vis_crate(self, i)
    }

    fn fold_vis_public(&mut self, i: syn::VisPublic) -> syn::VisPublic {
        syn::fold::fold_vis_public(self, i)
    }

    fn fold_vis_restricted(&mut self, i: syn::VisRestricted) -> syn::VisRestricted {
        syn::fold::fold_vis_restricted(self, i)
    }

    fn fold_visibility(&mut self, i: syn::Visibility) -> syn::Visibility {
        syn::fold::fold_visibility(self, i)
    }

    fn fold_where_clause(&mut self, i: syn::WhereClause) -> syn::WhereClause {
        syn::fold::fold_where_clause(self, i)
    }

    fn fold_where_predicate(&mut self, i: syn::WherePredicate) -> syn::WherePredicate {
        syn::fold::fold_where_predicate(self, i)
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
    // cmk use prettyplease::unparse;
    use crate::{transform_fn, DeltaPatType, UuidGenerator};
    // cmk remove from cargo use prettyplease::unparse;
    use quote::quote;
    use syn::{fold::Fold, parse_quote, parse_str, ItemFn, TypePath};

    fn str_to_type_path(s: &str) -> TypePath {
        parse_str(s).unwrap()
    }

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
    fn one_ndarray_usize_input() {
        let before = parse_quote! {
        pub fn any_slice_len(a: NdArrayLike<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }        };
        let expected = parse_quote! {
        pub fn any_slice_len<S0: Into<ndarray::ArrayView1<usize>>>(a: S0) -> Result<usize, anyhow::Error> {
            let a = a.into();
            let len = a.len();
            Ok(len)
        }}; //cmk0 missing the 'a

        let after = transform_fn(before, &mut generic_gen_test_factory());
        assert_item_fn_eq(&after, &expected);

        pub fn any_slice_len<'a, S0: Into<ndarray::ArrayView1<'a, usize>>>(
            a: S0,
        ) -> Result<usize, anyhow::Error> {
            let a = a.into();
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_slice_len([1, 2, 3].as_ref()).unwrap(), 3);
    }
}
