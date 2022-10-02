#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use input_like_derive::input_special;

    #[test]
    fn one_input() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_str_len1(s: StringLike) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
        assert!(any_str_len1("123")? == 3);
        Ok(())
    }

    #[test]
    fn two_inputs() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_str_len2(a: StringLike, b: StringLike) -> Result<usize, anyhow::Error> {
            let len = a.len() + b.len();
            Ok(len)
        }
        let s = "Hello".to_string();
        assert!(any_str_len2("123", s)? == 8);
        Ok(())
    }

    #[test]
    fn zero_inputs() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_str_len0() -> Result<usize, anyhow::Error> {
            let len = 0;
            Ok(len)
        }
        assert!(any_str_len0()? == 0);
        Ok(())
    }

    #[test]
    fn one_plus_two_input() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_str_len1plus2(
            a: usize,
            s: StringLike,
            b: usize,
        ) -> Result<usize, anyhow::Error> {
            let len = s.len() + a + b;
            Ok(len)
        }
        assert!(any_str_len1plus2(1, "123", 2)? == 6);
        Ok(())
    }

    #[test]
    fn one_path_input() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_count_path(p: PathLike) -> Result<usize, anyhow::Error> {
            let count = p.iter().count();
            Ok(count)
        }
        assert!(any_count_path(PathBuf::from("one/two/three"))? == 3);
        Ok(())
    }

    // #[test]
    // fn one_iter_usize_input() {
    //     let before = parse_quote! {
    //     pub fn any_count_iter(i: IterLike<usize>) -> Result<usize, anyhow::Error> {
    //         let count = i.count();
    //         Ok(count)
    //     }        };
    //     let expected = parse_quote! {
    //     pub fn any_count_iter<S0: IntoIterator<Item = usize>>(i: S0) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter();
    //         let count = i.count();
    //         Ok(count)
    //     }};

    //     let after = transform_fn(before, &mut generic_gen_test_factory());
    //     assert_item_fn_eq(&after, &expected);

    //     pub fn any_count_iter<S0: IntoIterator<Item = usize>>(
    //         i: S0,
    //     ) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter();
    //         let count = i.count();
    //         Ok(count)
    //     }
    //     assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    // }

    // #[test]
    // fn one_iter_i32() {
    //     let before = parse_quote! {
    //     pub fn any_count_iter(i: IterLike<i32>) -> Result<usize, anyhow::Error> {
    //         let count = i.count();
    //         Ok(count)
    //     }        };
    //     let expected = parse_quote! {
    //     pub fn any_count_iter<S0: IntoIterator<Item = i32>>(i: S0) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter();
    //         let count = i.count();
    //         Ok(count)
    //     }};

    //     let after = transform_fn(before, &mut generic_gen_test_factory());
    //     assert_item_fn_eq(&after, &expected);

    //     pub fn any_count_iter<S0: IntoIterator<Item = i32>>(i: S0) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter();
    //         let count = i.count();
    //         Ok(count)
    //     }
    //     assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    // }

    // #[test]
    // fn one_iter_t() {
    //     let before = parse_quote! {
    //     pub fn any_count_iter<T>(i: IterLike<T>) -> Result<usize, anyhow::Error> {
    //         let count = i.count();
    //         Ok(count)
    //     }        };
    //     let expected = parse_quote! {
    //     pub fn any_count_iter<T, S0: IntoIterator<Item = T>>(i: S0) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter();
    //         let count = i.count();
    //         Ok(count)
    //     }};

    //     let after = transform_fn(before, &mut generic_gen_test_factory());
    //     assert_item_fn_eq(&after, &expected);

    //     pub fn any_count_iter<T, S0: IntoIterator<Item = T>>(
    //         i: S0,
    //     ) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter();
    //         let count = i.count();
    //         Ok(count)
    //     }
    //     assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
    // }

    // #[test]
    // fn one_iter_path() {
    //     let before = parse_quote! {
    //     pub fn any_count_iter(i: IterLike<PathLike>) -> Result<usize, anyhow::Error> {
    //         let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
    //         Ok(sum_count)
    //     }        };
    //     let expected = parse_quote! {
    //     pub fn any_count_iter<S0: AsRef<std::path::Path>, S1: IntoIterator<Item = S0>>(
    //         i: S1
    //     ) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter(); // todo should the map be optional?
    //         let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
    //         Ok(sum_count)
    //     }};

    //     let after = transform_fn(before, &mut generic_gen_test_factory());
    //     assert_item_fn_eq(&after, &expected);

    //     pub fn any_count_iter<S0: AsRef<std::path::Path>, S1: IntoIterator<Item = S0>>(
    //         i: S1,
    //     ) -> Result<usize, anyhow::Error> {
    //         let i = i.into_iter(); // todo should the map be optional?
    //         let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
    //         Ok(sum_count)
    //     }
    //     assert_eq!(any_count_iter(["a/b", "d"]).unwrap(), 3);
    // }

    // #[test]
    // fn one_vec_path() {
    //     let before = parse_quote! {
    //     pub fn any_count_vec(
    //         i: Vec<PathLike>,
    //     ) -> Result<usize, anyhow::Error> {
    //         let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
    //         Ok(sum_count)
    //     }};
    //     let expected = parse_quote! {
    //     pub fn any_count_vec<S0: AsRef<std::path::Path>>(
    //         i: Vec<S0>
    //     ) -> Result<usize, anyhow::Error> {
    //         let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
    //         Ok(sum_count)
    //     }};

    //     let after = transform_fn(before, &mut generic_gen_test_factory());
    //     assert_item_fn_eq(&after, &expected);

    //     pub fn any_count_vec<S0: AsRef<std::path::Path>>(
    //         i: Vec<S0>,
    //     ) -> Result<usize, anyhow::Error> {
    //         let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
    //         Ok(sum_count)
    //     }
    //     assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
    // }

    // #[test]
    // fn fold_one_path() {
    //     // cmk 9 rules: parse_quote!
    //     // cmk 9 rules: use format!(quote!()) to generate strings of code
    //     // cmk 9 rules quote! is a nice way to display short ASTs on one line, too
    //     let before = parse_quote! {IterLike<PathLike> };
    //     println!("before: {}", quote!(before));
    //     let mut gen = generic_gen_test_factory();
    //     let mut struct1 = DeltaPatType {
    //         generic_params: vec![],
    //         generic_gen: &mut gen,
    //         last_special: None,
    //     };
    //     let result = struct1.fold_type(before);
    //     for generic_param in struct1.generic_params {
    //         println!("generic_param: {}", quote!(#generic_param));
    //     }

    //     println!("result: {}", quote!(#result));
    // }

    // #[test]
    // fn one_array_usize_input() {
    //     let before = parse_quote! {
    //     pub fn any_slice_len(a: ArrayLike<usize>) -> Result<usize, anyhow::Error> {
    //         let len = a.len();
    //         Ok(len)
    //     }        };
    //     let expected = parse_quote! {
    //     pub fn any_slice_len<S0: AsRef<[usize]>>(a: S0) -> Result<usize, anyhow::Error> {
    //         let a = a.as_ref();
    //         let len = a.len();
    //         Ok(len)
    //     }};

    //     let after = transform_fn(before, &mut generic_gen_test_factory());
    //     assert_item_fn_eq(&after, &expected);

    //     pub fn any_slice_len<S0: AsRef<[usize]>>(a: S0) -> Result<usize, anyhow::Error> {
    //         let a = a.as_ref();
    //         let len = a.len();
    //         Ok(len)
    //     }
    //     assert_eq!(any_slice_len([1, 2, 3]).unwrap(), 3);
    // }

    // #[test]
    // fn understand_lifetime_parse() {
    //     let a = Lifetime::new("'a", syn::__private::Span::call_site());
    //     println!("a: {}", quote!(#a));
    //     let b: Lifetime = parse_quote!('a);
    //     println!("b: {}", quote!(#b));

    //     let _generic_param: GenericParam = parse_quote!(S1: Into<ndarray::ArrayView1<'S0, S2>>);
    //     println!("gp: {}", quote!(#_generic_param));
    //     println!("done");
    // }

    #[test]
    fn one_ndarray_usize_input_x() {
        #[input_special]
        pub fn any_slice_len(a: NdArrayLike<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_slice_len([1, 2, 3].as_ref()).unwrap(), 3);
    }

    // cmk must test badly-formed functions to see that the error messages make sense.
}
