#![cfg(test)]

use crate::{anyinput_core, anyinput_core_sample};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    fold::Fold,
    parse2, parse_quote, parse_str,
    punctuated::Punctuated,
    token::{Comma, Lt},
    ItemFn, Stmt, WherePredicate,
};
#[cfg(feature = "ndarray")]
use syn::{GenericParam, Lifetime};

#[test]
fn first() {
    let before = quote! {
        fn hello_universe() {
            println!("Hello, universe!");
        }
    };

    let after = anyinput_core_sample(quote!(), before);
    assert_eq!(
        after.to_string(),
        "fn hello_world () { println ! (\"Hello, world!\") ; }"
    );
}

fn assert_tokens_eq(expected: &TokenStream, actual: &TokenStream) {
    let expected = expected.to_string();
    let actual = actual.to_string();

    if expected != actual {
        println!(
            "{}",
            colored_diff::PrettyDifference {
                expected: &expected,
                actual: &actual,
            }
        );
        println!("expected: {}", &expected);
        println!("actual  : {}", &actual);
        panic!("expected != actual");
    }
}

#[test]
fn one_input() {
    let before = quote! {
    fn any_str_len(s: AnyString) -> usize
    {
        s.len()
    }
    };

    let expected = quote! {
    fn any_str_len<AnyString0>(s: AnyString0) -> usize
    where
        AnyString0: AsRef<str>
    {
        let s = s.as_ref();
        s.len()
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    fn any_str_len<AnyString0>(s: AnyString0) -> usize
    where
        AnyString0: AsRef<str>,
    {
        let s = s.as_ref();
        s.len()
    }
    assert_eq!(any_str_len("abc"), 3);
}

#[test]
fn two_inputs() -> anyhow::Result<()> {
    let before = quote! {
        pub fn any_str_len(a: AnyString, b: AnyString) -> Result<usize, anyhow::Error> {
            let len = a.len() + b.len();
            Ok(len)
        }
    };
    let expected = quote! {
    pub fn any_str_len<AnyString0, AnyString1>(
        a: AnyString0,
        b: AnyString1
    ) -> Result<usize, anyhow::Error>
    where
        AnyString0: AsRef<str>,
        AnyString1: AsRef<str>
    {
        let b = b.as_ref();
        let a = a.as_ref();
        let len = a.len() + b.len();
        Ok(len)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_str_len<AnyString0, AnyString1>(
        a: AnyString0,
        b: AnyString1,
    ) -> Result<usize, anyhow::Error>
    where
        AnyString0: AsRef<str>,
        AnyString1: AsRef<str>,
    {
        let b = b.as_ref();
        let a = a.as_ref();
        let len = a.len() + b.len();
        Ok(len)
    }
    let s = "1234".to_string();
    assert_eq!(any_str_len("abc", s)?, 7);
    Ok(())
}

#[test]
fn zero_inputs() {
    let before = quote! {
    pub fn any_str_len0() -> Result<usize, anyhow::Error> {
        let len = 0;
        Ok(len)
    }};
    let expected = quote! {
    pub fn any_str_len0 () -> Result<usize, anyhow::Error> {
        let len = 0;
        Ok(len)
    }};

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);
}

#[test]
fn one_plus_two_input() -> anyhow::Result<()> {
    let before = quote! {
        pub fn any_str_len_plus2(a: usize, s: AnyString, b: usize) -> Result<usize, anyhow::Error> {
            let len = s.len()+a+b;
            Ok(len)
        }
    };
    let expected = quote! {
    pub fn any_str_len_plus2<AnyString0>(
        a: usize,
        s: AnyString0,
        b: usize
    ) -> Result<usize, anyhow::Error>
    where
        AnyString0: AsRef<str>
    {
        let s = s.as_ref();
        let len = s.len() + a + b;
        Ok(len)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_str_len_plus2<AnyString0>(
        a: usize,
        s: AnyString0,
        b: usize,
    ) -> Result<usize, anyhow::Error>
    where
        AnyString0: AsRef<str>,
    {
        let s = s.as_ref();
        let len = s.len() + a + b;
        Ok(len)
    }
    assert_eq!(any_str_len_plus2(1, "abc", 2)?, 6);
    Ok(())
}

#[test]
fn one_path_input() {
    let before = quote! {
    pub fn any_count_path(p: AnyPath) -> Result<usize, anyhow::Error> {
        let count = p.iter().count();
        Ok(count)
    }
      };

    let expected = quote! {
    pub fn any_count_path<AnyPath0>(p: AnyPath0) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>
    {
        let p = p.as_ref();
        let count = p.iter().count();
        Ok(count)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_path<AnyPath0>(p: AnyPath0) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>,
    {
        let p = p.as_ref();
        let count = p.iter().count();
        Ok(count)
    }
    assert_eq!(any_count_path("abc/ed").unwrap(), 2);
}

#[test]
fn one_iter_usize_input() {
    let before = quote! {
        pub fn any_count_iter(i: AnyIter<usize>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
    };
    let expected = quote! {
    pub fn any_count_iter<AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        AnyIter0: IntoIterator<Item = usize>
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        AnyIter0: IntoIterator<Item = usize>,
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
}

#[test]
fn one_iter_i32() {
    let before = quote! {
    pub fn any_count_iter(i: AnyIter<i32>) -> Result<usize, anyhow::Error> {
        let count = i.count();
        Ok(count)
    }
        };
    let expected = quote! {
    pub fn any_count_iter<AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        AnyIter0: IntoIterator<Item = i32>
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        AnyIter0: IntoIterator<Item = i32>,
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
}

#[test]
fn one_iter_t() {
    let before = quote! {
    pub fn any_count_iter<T>(i: AnyIter<T>) -> Result<usize, anyhow::Error> {
        let count = i.count();
        Ok(count)
    }
       };
    let expected = quote! {
    pub fn any_count_iter<T, AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        AnyIter0: IntoIterator<Item = T>
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<T, AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        AnyIter0: IntoIterator<Item = T>,
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
}

#[test]
fn one_iter_t_where() {
    let before = quote! {
    pub fn any_count_iter<T>(i: AnyIter<T>) -> Result<usize, anyhow::Error>
    where T: Copy
     {
        let count = i.count();
        Ok(count)
    }
       };
    let expected = quote! {
    pub fn any_count_iter<T, AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        T: Copy,
        AnyIter0: IntoIterator<Item = T>
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<T, AnyIter0>(i: AnyIter0) -> Result<usize, anyhow::Error>
    where
        T: Copy,
        AnyIter0: IntoIterator<Item = T>,
    {
        let i = i.into_iter();
        let count = i.count();
        Ok(count)
    }
    assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
}
#[test]
fn one_iter_path() {
    let before = quote! {
    pub fn any_count_iter(i: AnyIter<AnyPath>) -> Result<usize, anyhow::Error> {
        let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
       };
    let expected = quote! {
    pub fn any_count_iter<AnyPath0, AnyIter1>(i: AnyIter1) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>,
        AnyIter1: IntoIterator<Item = AnyPath0>
    {
        let i = i.into_iter();
        let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<AnyPath0, AnyIter1>(i: AnyIter1) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>,
        AnyIter1: IntoIterator<Item = AnyPath0>,
    {
        let i = i.into_iter();
        let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
    assert_eq!(any_count_iter(["a/b", "d"]).unwrap(), 3);
}

#[test]
fn one_vec_path() {
    let before = quote! {
        pub fn any_count_vec(
            i: Vec<AnyPath>,
        ) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
    };
    let expected = quote! {
        pub fn any_count_vec<AnyPath0>(i: Vec<AnyPath0>) -> Result<usize, anyhow::Error>
        where
            AnyPath0: AsRef<std::path::Path>
        {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_vec<AnyPath0>(i: Vec<AnyPath0>) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>,
    {
        let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
    assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
}

#[test]
fn one_array_usize_input() {
    let before = quote! {
    pub fn any_array_len(a: AnyArray<usize>) -> Result<usize, anyhow::Error> {
        let len = a.len();
        Ok(len)
    }
      };
    let expected = quote! {
    pub fn any_array_len<AnyArray0>(a: AnyArray0) -> Result<usize, anyhow::Error>
    where
        AnyArray0: AsRef<[usize]>
    {
        let a = a.as_ref();
        let len = a.len();
        Ok(len)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_array_len<AnyArray0>(a: AnyArray0) -> Result<usize, anyhow::Error>
    where
        AnyArray0: AsRef<[usize]>,
    {
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
    let before = quote! {
    pub fn any_array_len(a: AnyNdArray<usize>) -> Result<usize, anyhow::Error> {
        let len = a.len();
        Ok(len)
    }        };
    let expected = quote! {
    pub fn any_array_len<'any_nd_array1, AnyNdArray0>(
        a: AnyNdArray0
    ) -> Result<usize, anyhow::Error>
    where
        AnyNdArray0: Into<ndarray::ArrayView1<'any_nd_array1, usize> >
    {
        let a = a.into();
        let len = a.len();
        Ok(len)
    }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    // The lines are long enough that Clippy would like a comma after
    // a:AnyNdArray0, but the macro doesn't do that because
    // it doesn't know the line length.
    pub fn any_array_len<'any_nd_array1, AnyNdArray0>(
        a: AnyNdArray0,
    ) -> Result<usize, anyhow::Error>
    where
        AnyNdArray0: Into<ndarray::ArrayView1<'any_nd_array1, usize>>,
    {
        let a = a.into();
        let len = a.len();
        Ok(len)
    }
    assert_eq!(any_array_len([1, 2, 3].as_ref()).unwrap(), 3);
}

#[test]
#[cfg(feature = "ndarray")]
fn complex() {
    let before = quote! {
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
    let expected = quote! {
    pub fn complex_total<'any_nd_array4, AnyPath0, AnyArray1, AnyIter2, AnyNdArray3>(
        a: usize,
        b: AnyIter2,
        c: AnyNdArray3
    ) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>,
        AnyArray1: AsRef<[AnyPath0]>,
        AnyIter2: IntoIterator<Item = Vec<AnyArray1> >,
        AnyNdArray3: Into<ndarray::ArrayView1<'any_nd_array4, usize> >
    {
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

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn complex_total<'any_nd_array4, AnyPath0, AnyArray1, AnyIter2, AnyNdArray3>(
        a: usize,
        b: AnyIter2,
        c: AnyNdArray3,
    ) -> Result<usize, anyhow::Error>
    where
        AnyPath0: AsRef<std::path::Path>,
        AnyArray1: AsRef<[AnyPath0]>,
        AnyIter2: IntoIterator<Item = Vec<AnyArray1>>,
        AnyNdArray3: Into<ndarray::ArrayView1<'any_nd_array4, usize>>,
    {
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
    let before = quote! {
    fn len_plus_2(s: AnyString) -> Result<usize, anyhow::Error> {
        Ok(s.len()+2)
    }        };
    let after = anyinput_core(quote!(), before);
    println!("after: {}", quote! { #after});
    let expected = quote! {
    fn len_plus_2<AnyString0>(s: AnyString0) -> Result<usize, anyhow::Error>
    where
        AnyString0: AsRef<str>
    {
        let s = s.as_ref();
        Ok(s.len() + 2)
    }
    };
    assert_tokens_eq(&expected, &after);

    fn len_plus_2<AnyString0>(s: AnyString0) -> Result<usize, anyhow::Error>
    where
        AnyString0: AsRef<str>,
    {
        let s = s.as_ref();
        Ok(s.len() + 2)
    }
    assert_eq!(len_plus_2("hello")?, 7);

    Ok(())
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_1() {
    let before = quote! {
    pub fn any_str_len(s: AnyIter<AnyString,usize>) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_2() {
    let before = quote! {
    pub fn any_str_len(s: AnyIter<3>) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_3() {
    let before = quote! {
    pub fn any_str_len(s: AnyIter(AnyString)) {
        s.len()
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_4() {
    let before = quote! {
    pub fn any_str_len(s: AnyArray) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_5() {
    let before = quote! {
    pub fn any_str_len(s: AnyIter) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}
#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_6() {
    let before = quote! {
    pub fn any_str_len(s: AnyNdArray) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_7() {
    let before = quote! {
    pub fn any_str_len(s: AnyString<usize>) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
#[should_panic(
    expected = "proc-macro-error API cannot be used outside of `entry_point` invocation, perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
)]
fn one_bad_input_8() {
    let before = quote! {
    pub fn any_str_len(s: AnyPath<usize>) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let _after = anyinput_core(quote!(), before);
}

#[test]
fn see_bed_reader() {
    let before = quote! {
     pub fn iid(mut self, iid: AnyIter<AnyString>) -> Self {
         // Unwrap will always work because BedBuilder starting with some metadata
         self.metadata.as_mut().unwrap().set_iid(iid);
         self
     }
    };
    let after = anyinput_core(quote!(), before);
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

#[test]
fn understand_token_stream() {
    let token_stream = quote!(
        pub fn hello() {
            println!("hello world")
        }
    );
    println!("{:#?}", token_stream);
}

#[test]
fn understand_parse_quote() {
    let item_fn: ItemFn = parse_quote!(
        pub fn hello() {
            println!("hello world")
        }
    );

    // println!("{}", &item_fn);
    println!("{:?}", &item_fn);
    println!("{:#?}", &item_fn);

    use quote::ToTokens;
    let token_stream: proc_macro2::TokenStream = item_fn.clone().into_token_stream();
    let _token_stream2: proc_macro2::TokenStream = quote!(#item_fn);
    println!("{}", &token_stream);
    println!("{:?}", &token_stream);
    println!("{:#?}", &token_stream);
}

/// Also see https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=7ae7d8fd405a2af5afc60ef3de9b2dad
#[test]
fn conversion_combinations() {
    // Literal code to tokens, syntax, and string-of-code
    let tokens1 = quote! {
        fn hello() {
            println!("hello world")
        }
    };
    let syntax1: ItemFn = parse_quote! {
        fn hello() {
            println!("hello world")
        }
    };
    let string_of_code1 = stringify!(
        fn hello() {
            println!("hello world")
        }
    );
    assert_eq!(string_of_code1, "fn hello() { println! (\"hello world\") }");

    // Tokens to string-of-code & string-of-tokens
    assert_eq!(
        tokens1.to_string(),
        "fn hello () { println ! (\"hello world\") }"
    );
    //  -- use {:#?} for the pretty-printed version
    let string_of_tokens1 = format!("{:?}", tokens1);
    assert!(string_of_tokens1.starts_with("TokenStream [Ident { sym: fn }"));

    // Syntax to string-of-code & string-of-syntax
    assert_eq!(
        quote!(#syntax1).to_string(),
        "fn hello () { println ! (\"hello world\") }"
    );
    //  -- use {:#?} for the pretty-printed version
    let string_of_syntax1 = format!("{:?}", syntax1);
    assert!(string_of_syntax1.starts_with("ItemFn { attrs: [], "));

    // Tokens <--> syntax
    let syntax2_result: Result<ItemFn, syn::Error> = parse2::<ItemFn>(tokens1);
    let syntax2: ItemFn = syntax2_result.expect("todo: need better error");
    let _tokens2 = quote!(#syntax2); // or .into_token_stream()

    // String of code to syntax or tokens
    let syntax3_result: Result<ItemFn, syn::Error> =
        parse_str("fn hello () { println ! (\"hello world\") }");
    let _syntax3 = syntax3_result.expect("todo: need better error");
    let tokens3_result: Result<TokenStream, syn::Error> =
        parse_str("fn hello () { println ! (\"hello world\") }");
    let _tokens3 = tokens3_result.expect("todo: need better error");

    // Literal code to string-of-tokens and string-of-syntax
    assert!(format!(
        "{:?}",
        quote! {
            fn hello() {
                println!("hello world")
            }
        }
    )
    .starts_with("TokenStream [Ident { sym: fn }"));
    assert!(format!(
        "{:?}",
        parse2::<ItemFn>(quote! {
            fn hello() {
                println!("hello world")
            }
        })
        .expect("todo: need better error")
    )
    .starts_with("ItemFn { attrs: [], "));
}

/// Also see https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=acce21f0c87ce66cb90c4cf12103e247
struct StmtCounter {
    count: usize,
}

impl Fold for StmtCounter {
    fn fold_stmt(&mut self, stmt_old: Stmt) -> Stmt {
        let stmt_middle = syn::fold::fold_stmt(self, stmt_old);

        println!("stmt #{}: {}", self.count, quote!(#stmt_middle));
        self.count += 1;

        if quote!(#stmt_middle).to_string().contains("galaxy") {
            parse_quote!(println!("hello universe");)
        } else {
            stmt_middle
        }
    }
}

#[test]
fn count_statements() {
    let mut stmt_counter = StmtCounter { count: 0 };
    let item_fn_old: ItemFn = parse_quote! {
        fn hello() {
            println!("hello world");
            {
                println!("hello solar system");
                println!("hello galaxy");
            }
        }
    };
    let item_fn_new = stmt_counter.fold_item_fn(item_fn_old);
    println!("item_fn_new: {}", quote!(#item_fn_new));
    println!("count: {}", stmt_counter.count);
}

#[test]
fn parse_quote_sample() {
    let _lt: Lt = parse_quote!(<);
    let _where_list: Punctuated<WherePredicate, Comma> = parse_quote!();
}
