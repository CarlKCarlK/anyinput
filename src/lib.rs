// cmk rename to "anyinput"
// cmk test what happens when you apply to non-functions (e.g. struct)
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

    #[test]
    fn one_iter_usize_input() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_count_iter(i: IterLike<usize>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_iter_i32() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_count_iter(i: IterLike<i32>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_iter_t() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_count_iter<T>(i: IterLike<T>) -> Result<usize, anyhow::Error> {
            let count = i.count();
            Ok(count)
        }
        assert_eq!(any_count_iter([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_iter_path() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_count_iter(i: IterLike<PathLike>) -> Result<usize, anyhow::Error> {
            let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_iter(["a/b", "d"]).unwrap(), 3);
        Ok(())
    }

    // cmk rename UU generator from U* to T* -- also test generator
    #[test]
    fn one_vec_path() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_count_vec(i: Vec<PathLike>) -> Result<usize, anyhow::Error> {
            let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
            Ok(sum_count)
        }
        assert_eq!(any_count_vec(vec!["a/b", "d"]).unwrap(), 3);
        Ok(())
    }

    #[test]
    fn one_array_usize_input() -> Result<(), anyhow::Error> {
        #[input_special]
        pub fn any_slice_len(a: ArrayLike<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_slice_len([1, 2, 3]).unwrap(), 3);
        Ok(())
    }

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
