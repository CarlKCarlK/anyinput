use anyinput::anyinput;
use std::path::PathBuf;

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
    assert_eq!(any_count_iter([1, 2, 3])?, 3);
    Ok(())
}

#[test]
fn one_iter_i32() -> Result<(), anyhow::Error> {
    #[anyinput]
    pub fn any_count_iter(i: AnyIter<i32>) -> Result<usize, anyhow::Error> {
        let count = i.count();
        Ok(count)
    }
    assert_eq!(any_count_iter([1, 2, 3])?, 3);
    Ok(())
}

#[test]
fn one_iter_t() -> Result<(), anyhow::Error> {
    #[anyinput]
    pub fn any_count_iter<T>(i: AnyIter<T>) -> Result<usize, anyhow::Error> {
        let count = i.count();
        Ok(count)
    }
    assert_eq!(any_count_iter([1, 2, 3])?, 3);
    Ok(())
}

#[test]
fn one_iter_path() -> Result<(), anyhow::Error> {
    #[anyinput]
    pub fn any_count_iter(i: AnyIter<AnyPath>) -> Result<usize, anyhow::Error> {
        let sum_count = i.map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
    assert_eq!(any_count_iter(["a/b", "d"])?, 3);
    Ok(())
}

#[test]
fn one_vec_path() -> Result<(), anyhow::Error> {
    #[anyinput]
    pub fn any_count_vec(i: Vec<AnyPath>) -> Result<usize, anyhow::Error> {
        let sum_count = i.iter().map(|x| x.as_ref().iter().count()).sum();
        Ok(sum_count)
    }
    assert_eq!(any_count_vec(vec!["a/b", "d"])?, 3);
    Ok(())
}

#[cfg(feature = "ndarray")]
#[test]
fn one_array_usize_input() -> Result<(), anyhow::Error> {
    #[anyinput]
    pub fn any_array_len(a: AnyArray<usize>) -> Result<usize, anyhow::Error> {
        let len = a.len();
        Ok(len)
    }
    assert_eq!(any_array_len([1, 2, 3])?, 3);
    Ok(())
}

#[cfg(feature = "ndarray")]
#[test]
fn one_ndarray_usize_input() -> Result<(), anyhow::Error> {
    use ndarray;

    #[anyinput]
    pub fn any_array_len(a: AnyNdArray<usize>) -> Result<usize, anyhow::Error> {
        let len = a.len();
        Ok(len)
    }
    assert_eq!(any_array_len([1, 2, 3].as_ref())?, 3);
    Ok(())
}

#[cfg(feature = "ndarray")]
#[test]
fn complex() -> Result<(), anyhow::Error> {
    use ndarray;

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
    assert_eq!(complex_total(17, [vec![["one"]]], [1, 2, 3].as_ref())?, 24);
    Ok(())
}

#[test]
#[cfg(feature = "ndarray")]
fn doc_ndarray() -> Result<(), anyhow::Error> {
    use anyhow::Result;
    use ndarray;

    #[anyinput]
    fn any_mean(array: AnyNdArray<f32>) -> Result<f32, anyhow::Error> {
        if let Some(mean) = array.mean() {
            Ok(mean)
        } else {
            Err(anyhow::anyhow!("empty array"))
        }
    }

    // 'AnyNdArray' works with any 1-D array-like thing, but must be borrowed.
    assert_eq!(any_mean(&[10.0, 20.0, 30.0, 40.0])?, 25.0);
    assert_eq!(any_mean(&ndarray::array![10.0, 20.0, 30.0, 40.0])?, 25.0);

    Ok(())
}

#[test]
fn doc_path() -> Result<(), anyhow::Error> {
    use crate::anyinput;
    use anyhow::Result;
    use std::path::Path;

    #[anyinput]
    fn component_count(path: AnyPath) -> Result<usize, anyhow::Error> {
        let count = path.iter().count();
        Ok(count)
    }

    // By using AnyPath, component_count works with any
    // string-like or path-like thing, borrowed or moved.
    assert_eq!(component_count("usr/files/home")?, 3);
    let path = Path::new("usr/files/home");
    let pathbuf = path.to_path_buf();
    assert_eq!(component_count(&path)?, 3);
    assert_eq!(component_count(pathbuf)?, 3);

    Ok(())
}

#[test]
fn doc_iter() -> Result<(), anyhow::Error> {
    use crate::anyinput;
    use anyhow::Result;

    #[anyinput]
    fn two_iterator_sum(
        iter1: AnyIter<usize>,
        iter2: AnyIter<AnyString>,
    ) -> Result<usize, anyhow::Error> {
        let mut sum = iter1.sum();
        for any_string in iter2 {
            // Needs .as_ref to turn the nested AnyString into a &str.
            sum += any_string.as_ref().len();
        }
        Ok(sum)
    }
    assert_eq!(two_iterator_sum(1..=10, ["a", "bb", "ccc"])?, 61);
    Ok(())
}

#[test]
fn doc_array() -> Result<(), anyhow::Error> {
    use crate::anyinput;
    use anyhow::Result;

    #[anyinput]
    fn indexed_component_count(
        array: AnyArray<AnyPath>,
        index: usize,
    ) -> Result<usize, anyhow::Error> {
        // Needs .as_ref to turn the nested AnyPath into a &Path.
        let path = array[index].as_ref();
        let count = path.iter().count();
        Ok(count)
    }
    assert_eq!(
        indexed_component_count(vec!["usr/files/home", "usr/data"], 1)?,
        2
    );
    Ok(())
}

#[test]
#[cfg(feature = "ndarray")]
fn doc_ndarray2() -> Result<(), anyhow::Error> {
    use crate::anyinput;
    use anyhow::Result;
    use ndarray;

    #[anyinput]
    fn any_mean(array: AnyNdArray<f32>) -> Result<f32, anyhow::Error> {
        if let Some(mean) = array.mean() {
            Ok(mean)
        } else {
            Err(anyhow::anyhow!("empty array"))
        }
    }

    // 'AnyNdArray' works with any 1-D array-like thing, but must be borrowed.
    assert_eq!(any_mean(&[10.0, 20.0, 30.0, 40.0])?, 25.0);
    assert_eq!(any_mean(&ndarray::array![10.0, 20.0, 30.0, 40.0])?, 25.0);
    Ok(())
}

#[test]
fn more_path() -> Result<(), anyhow::Error> {
    use crate::anyinput;
    use anyhow::Result;
    use std::path::Path;

    fn component_count1(path: impl AsRef<Path>) -> Result<usize, anyhow::Error> {
        let path = path.as_ref();
        Ok(path.iter().count())
    }
    assert_eq!(component_count1("usr/files/home")?, 3);

    #[anyinput]
    fn component_count2(path: AnyPath) -> Result<usize, anyhow::Error> {
        Ok(path.iter().count())
    }
    assert_eq!(component_count2("usr/files/home")?, 3);

    Ok(())
}

#[test]
fn more_string() -> Result<(), anyhow::Error> {
    use std::borrow::Borrow;

    pub fn len_plus_2(s: impl Borrow<str>) -> usize {
        s.borrow().len() + 2
    }

    assert_eq!(len_plus_2("Hello"), 7); // move a &str
                                        // let input: &str = "Hello";
                                        // assert_eq!(len_plus_2(&input), 7); // borrow a &str
                                        // let input: String = "Hello".to_string();
                                        // assert_eq!(len_plus_2(&input), 7); // borrow a String
                                        // let input2: &String = &input;
                                        // assert_eq!(len_plus_2(&input2), 7); // borrow a &String
                                        // assert_eq!(len_plus_2(input2), 7); // move a &String
                                        // assert_eq!(len_plus_2(input), 7); // move a String
    Ok(())
}

#[test]
fn more_iter() -> Result<(), anyhow::Error> {
    use crate::anyinput;
    use std::borrow::Borrow;

    pub fn two_iterator_sum(
        iter1: impl IntoIterator<Item = usize>,
        iter2: impl IntoIterator<Item = impl Borrow<str>>,
    ) -> usize {
        let mut sum = iter1.into_iter().sum();

        for string in iter2 {
            sum += string.borrow().len();
        }

        sum
    }

    two_iterator_sum(1..=10, ["a", "bb", "ccc"]);
    let s0 = "bb".to_string();
    // two_iterator_sum(1..=10, [&s0]);

    #[anyinput]
    pub fn two_iterator_sum2(iter1: AnyIter<usize>, iter2: AnyIter<AnyString>) -> usize {
        let mut sum = iter1.sum();

        for string in iter2 {
            sum += string.as_ref().len();
        }

        sum
    }
    two_iterator_sum2(1..=10, [&s0]);

    Ok(())
}

#[test]
fn more_iter2() -> Result<(), anyhow::Error> {
    use crate::anyinput;

    #[anyinput]
    fn iid0(iid: AnyIter<AnyString>) -> usize {
        let mut sum = 0;
        for s in iid {
            sum += s.as_ref().len();
        }
        sum
    }

    fn iid1<I: IntoIterator<Item = T>, T: AsRef<str>>(iid: I) -> usize {
        let mut sum = 0;
        for s in iid {
            sum += s.as_ref().len();
        }
        sum
    }

    fn iid2<I: IntoIterator>(iid: I) -> usize
    where
        <I as IntoIterator>::Item: AsRef<str>,
    {
        let mut sum = 0;
        for s in iid {
            sum += s.as_ref().len();
        }
        sum
    }

    //let fa = iid0;

    assert_eq!(iid0::<&str, [&str; 3]>(["a", "bb", "ccc"]), 6);

    // assert_eq!(iid0::<_, _>(["a", "bb", "ccc"]), 6);
    assert_eq!(iid1::<[&str; 3], &str>(["a", "bb", "ccc"]), 6);
    assert_eq!(iid2::<[&str; 3]>(["a", "bb", "ccc"]), 6);
    assert_eq!(iid1(["a", "bb", "ccc"]), 6);
    assert_eq!(iid2(["a", "bb", "ccc"]), 6);

    Ok(())
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

// todo make this a real test
// #[test]
// fn misapply() -> Result<(), anyhow::Error> {
//     use crate::anyinput;

//     #[anyinput]
//     struct Test {
//         a: AnyString,
//         b: AnyString,
//     }
//     Ok(())
// }

// todo later should their be a warning/error if there is no Anyinput on a function to which this has been applied?
// todo must test badly-formed functions to see that the error messages make sense.
