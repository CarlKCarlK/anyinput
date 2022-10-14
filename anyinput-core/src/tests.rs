#![cfg(test)]

use crate::{anyinput_core, generic_gen_simple_factory, DeltaPatType};
use quote::{quote, ToTokens};
use syn::{fold::Fold, parse_quote, ItemFn};
#[cfg(feature = "ndarray")]
use syn::{GenericParam, Lifetime};

fn assert_tokens_eq<T0: ToTokens, T1: ToTokens>(expected: T0, actual: T1) {
    let expected = expected.to_token_stream().to_string();
    let actual = actual.to_token_stream().to_string();

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
fn one_input() -> anyhow::Result<()> {
    let before = quote! {
    pub fn any_str_len(s: AnyString) -> Result<usize, anyhow::Error> {
        let len = s.len();
        Ok(len)
    }
       };
    let expected = quote! {
        pub fn any_str_len<AnyString0: AsRef<str> >(s: AnyString0) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len();
            Ok(len)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_str_len<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
        let s = s.as_ref();
        let len = s.len();
        Ok(len)
    }
    assert_eq!(any_str_len("abc")?, 3);

    Ok(())
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
        pub fn any_str_len<AnyString0: AsRef<str>, AnyString1: AsRef<str> >(
            a: AnyString0,
            b: AnyString1
        ) -> Result<usize, anyhow::Error> {
            let b = b.as_ref();
            let a = a.as_ref();
            let len = a.len() + b.len();
            Ok(len)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_str_len<AnyString0: AsRef<str>, AnyString1: AsRef<str>>(
        a: AnyString0,
        b: AnyString1,
    ) -> Result<usize, anyhow::Error> {
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
        pub fn any_str_len_plus2<AnyString0: AsRef<str> >(
            a: usize,
            s: AnyString0,
            b: usize
        ) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            let len = s.len() + a + b;
            Ok(len)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_str_len_plus2<AnyString0: AsRef<str>>(
        a: usize,
        s: AnyString0,
        b: usize,
    ) -> Result<usize, anyhow::Error> {
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
        pub fn any_count_path<AnyPath0: AsRef<std::path::Path> >(
            p: AnyPath0
        ) -> Result<usize, anyhow::Error> {
            let p = p.as_ref();
            let count = p.iter().count();
            Ok(count)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_path<AnyPath0: AsRef<std::path::Path>>(
        p: AnyPath0,
    ) -> Result<usize, anyhow::Error> {
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
        pub fn any_count_iter<AnyIter0: IntoIterator<Item = usize> >(
            i: AnyIter0
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<AnyIter0: IntoIterator<Item = usize>>(
        i: AnyIter0,
    ) -> Result<usize, anyhow::Error> {
        let i: <AnyIter0 as IntoIterator>::IntoIter = i.into_iter();
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
        pub fn any_count_iter<AnyIter0: IntoIterator<Item = i32> >(
            i: AnyIter0
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<AnyIter0: IntoIterator<Item = i32>>(
        i: AnyIter0,
    ) -> Result<usize, anyhow::Error> {
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
        pub fn any_count_iter<T, AnyIter0: IntoIterator<Item = T> >(
            i: AnyIter0
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let count = i.count();
            Ok(count)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<T, AnyIter0: IntoIterator<Item = T>>(
        i: AnyIter0,
    ) -> Result<usize, anyhow::Error> {
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
            pub fn any_count_iter<
            AnyPath0: AsRef<std::path::Path>,
            AnyIter1: IntoIterator<Item = AnyPath0>
        >(
            i: AnyIter1
        ) -> Result<usize, anyhow::Error> {
            let i = i.into_iter();
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_iter<
        AnyPath0: AsRef<std::path::Path>,
        AnyIter1: IntoIterator<Item = AnyPath0>,
    >(
        i: AnyIter1,
    ) -> Result<usize, anyhow::Error> {
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
    pub fn any_count_vec<AnyPath0: AsRef<std::path::Path> >(
        i: Vec<AnyPath0>
    ) -> Result<usize, anyhow::Error> {
        let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }};

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_count_vec<AnyPath0: AsRef<std::path::Path>>(
        i: Vec<AnyPath0>,
    ) -> Result<usize, anyhow::Error> {
        let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
    assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
}

#[test]
fn fold_one_path() {
    let before = parse_quote! {AnyIter<AnyPath> };
    println!("before: {}", quote!(before));
    let mut gen = generic_gen_simple_factory();
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
    let before = quote! {
    pub fn any_array_len(a: AnyArray<usize>) -> Result<usize, anyhow::Error> {
        let len = a.len();
        Ok(len)
    }
      };
    let expected = quote! {
        pub fn any_array_len<AnyArray0: AsRef<[usize]> >(
            a: AnyArray0
        ) -> Result<usize, anyhow::Error> {
            let a = a.as_ref();
            let len = a.len();
            Ok(len)
        }
    };

    let after = anyinput_core(quote!(), before);
    assert_tokens_eq(&expected, &after);

    pub fn any_array_len<AnyArray0: AsRef<[usize]>>(a: AnyArray0) -> Result<usize, anyhow::Error> {
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
            pub fn any_array_len<
            'any_nd_array1,
            AnyNdArray0: Into<ndarray::ArrayView1<'any_nd_array1, usize> > >(
            a: AnyNdArray0
        ) -> Result<usize, anyhow::Error> {
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
    pub fn any_array_len<
        'any_nd_array1,
        AnyNdArray0: Into<ndarray::ArrayView1<'any_nd_array1, usize>>,
    >(
        a: AnyNdArray0,
    ) -> Result<usize, anyhow::Error> {
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
        pub fn complex_total<
        'any_nd_array4,
        AnyPath0: AsRef<std::path::Path>,
        AnyArray1: AsRef<[AnyPath0]>,
        AnyIter2: IntoIterator<Item = Vec<AnyArray1> >,
        AnyNdArray3: Into<ndarray::ArrayView1<'any_nd_array4, usize > > >(
        a: usize,
        b: AnyIter2,
        c: AnyNdArray3
    ) -> Result<usize, anyhow::Error> {
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

    pub fn complex_total<
        'any_nd_array4,
        AnyPath0: AsRef<std::path::Path>,
        AnyArray1: AsRef<[AnyPath0]>,
        AnyIter2: IntoIterator<Item = Vec<AnyArray1>>,
        AnyNdArray3: Into<ndarray::ArrayView1<'any_nd_array4, usize>>,
    >(
        a: usize,
        b: AnyIter2,
        c: AnyNdArray3,
    ) -> Result<usize, anyhow::Error> {
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
        fn len_plus_2<AnyString0: AsRef<str> >(s: AnyString0) -> Result<usize, anyhow::Error> {
            let s = s.as_ref();
            Ok(s.len() + 2)
        }
    };
    assert_tokens_eq(&expected, &after);

    fn len_plus_2<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
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
    pub fn any_str_len(s: AnyIter(AnyString)) -> Result<usize, anyhow::Error> {
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
