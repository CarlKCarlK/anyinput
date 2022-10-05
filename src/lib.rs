#![doc = include_str!("../README.md")]
// cmk test what happens when you apply to non-functions (e.g. struct)
#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use anyinput_derive::anyinput;

    #[test]
    fn one_input() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_str_len1(s: AnyString) -> Result<usize, anyhow::Error> {
            let len = s.len();
            Ok(len)
        }
        assert!(any_str_len1("123")? == 3);
        Ok(())
    }

    #[test]
    fn two_inputs() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_str_len2(a: AnyString, b: AnyString) -> Result<usize, anyhow::Error> {
            let len = a.len() + b.len();
            Ok(len)
        }
        let s = "Hello".to_string();
        assert!(any_str_len2("123", s)? == 8);
        Ok(())
    }

    #[test]
    fn zero_inputs() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_str_len0() -> Result<usize, anyhow::Error> {
            let len = 0;
            Ok(len)
        }
        assert!(any_str_len0()? == 0);
        Ok(())
    }

    #[test]
    fn one_plus_two_input() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_str_len1plus2(a: usize, s: AnyString, b: usize) -> Result<usize, anyhow::Error> {
            let len = s.len() + a + b;
            Ok(len)
        }
        assert!(any_str_len1plus2(1, "123", 2)? == 6);
        Ok(())
    }

    #[test]
    fn one_path_input() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_count_path(p: AnyPath) -> Result<usize, anyhow::Error> {
            let count = p.iter().count();
            Ok(count)
        }
        assert!(any_count_path(PathBuf::from("one/two/three"))? == 3);
        Ok(())
    }

    #[test]
    fn one_iter_usize_input() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_count_iter(i: AnyIter<usize>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_iter_i32() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_count_iter(i: AnyIter<i32>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_iter_t() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_count_iter<T>(i: AnyIter<T>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_iter_path() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_count_iter(i: AnyIter<AnyPath>) -> Result<usize, anyhow::Error> {
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_iter(["a/b", "d"]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_vec_path() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_count_vec(i: Vec<AnyPath>) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_array_usize_input() -> Result<(), anyhow::Error> {
        #[anyinput]
        pub fn any_array_len(a: AnyArray<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_array_len([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    // cmk remove unwrap from examples and use ?

    #[test]
    fn one_ndarray_usize_input() {
        #[anyinput]
        pub fn any_array_len(a: AnyNdArray<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_array_len([1, 2, 3].as_ref()).unwrap(), 3);
    }
    // cmk remove "slice" from examples vocabulary

    // cmk in readme.md mention that you'll get nice VC hints for the type.
    // cmk add option into anyinput for long variables
    #[test]
    fn complex() {
        #[anyinput]
        pub fn complex_total(
            a: usize,
            b: AnyIter<Vec<AnyArray<AnyPath>>>,
            c: AnyNdArray<usize>,
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
        assert_eq!(
            complex_total(17, [vec![["one"]]], [1, 2, 3].as_ref()).unwrap(),
            24
        );
    }

    // cmk must test badly-formed functions to see that the error messages make sense.
    // cmk is there a nice way to diff the output vs. the expected output?
    // cmk rename helper as core
    // cmk see for an example readme telling folks they likely want the main crate https://github.com/colin-kiegel/rust-derive-builder/tree/master/derive_builder_macro
}

// cmk understand this test from https://github.com/dtolnay/quote/blob/master/tests/test.rs
// #[test]
// fn test_substitution() {
//     let x = X;
//     let tokens = quote!(#x <#x> (#x) [#x] {#x});

//     let expected = "X < X > (X) [X] { X }";

//     assert_eq!(expected, tokens.to_string());
// }
