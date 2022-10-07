anyinput
==========

cmk[<img alt="github" src="https://img.shields.io/badge/github-bed--reader-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/fastlmm/bed-reader)
[<img alt="crates.io" src="https://img.shields.io/crates/v/bed-reader.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/bed-reader)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-bed--reader-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/bed-reader)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/fastlmm/bed-reader/CI/master?style=for-the-badge" height="20">](https://github.com/fastlmm/bed-reader/actions?query=branch%3Amaster)

Easily create functions that accept any type of string, path, iterator-like, or array-line inputs.

##### Contents
- [anyinput](#anyinput)
        - [Contents](#contents)
  - [Examples](#examples)
  - [AnyInputs](#anyinputs)
  - [Notes & Features](#notes--features)
  - [How It Works](#how-it-works)
  - [Project Links cmk update](#project-links-cmk-update)


Examples
--------

Create a function that adds 2 to the length of any string-like thing.

```rust
use anyinput::anyinput;
use anyhow::Result;

#[anyinput]
fn len_plus_2(s: AnyString) -> Result<usize, anyhow::Error> {
    Ok(s.len()+2)
}

// By using AnyString, len_plus_2 works with
// &str, String, or &String -- borrowed or moved.
assert_eq!(len_plus_2("Hello")?, 7); // move a &str
let input: &str = "Hello";
assert_eq!(len_plus_2(&input)?, 7); // borrow a &str
let input: String = "Hello".to_string();
assert_eq!(len_plus_2(&input)?, 7); // borrow a String
let input2: &String = &input;
assert_eq!(len_plus_2(&input2)?, 7); // borrow a &String
assert_eq!(len_plus_2(input2)?, 7); // move a &String
assert_eq!(len_plus_2(input)?, 7); // move a String
# // '#' needed for doctest
# Ok::<(), anyhow::Error>(())
```

cmk format this code

Create a function that counts the components of any path-like thing.

```rust
use anyinput::anyinput;
use anyhow::Result;
use std::path::{PathBuf,Path};

#[anyinput]
fn component_count(path: AnyPath) -> Result<usize, anyhow::Error> {
    let count = path.iter().count();
    Ok(count)
}

// By using AnyPath, component_count works with any
// string-like or path-like thing, borrowed or moved.
assert_eq!(component_count("usr/files/home")?, 3);
let path = Path::new("usr/files/home");
assert_eq!(component_count(&path)?, 3);
assert_eq!(component_count(path.to_path_buf())?, 3);
# // '#' needed for doctest
# Ok::<(), anyhow::Error>(())
```

Nesting and multiple AnyInputs are allowed. Here we create a function with two inputs. One input accepts any iterator-like
thing of `usize`. The second input accepts any iterator-like thing of string-like things. The function returns the sum of the numbers and string lengths.

```rust
use anyinput::anyinput;
use anyhow::Result;

#[anyinput]
fn two_iterator_sum(iter1: AnyIter<usize>, iter2: AnyIter<AnyString>) -> Result<usize, anyhow::Error> {
    let mut sum = iter1.sum();
    for any_string in iter2
    {
        // Needs .as_ref to turn the nested AnyString into a &str.
        sum += any_string.as_ref().len();
    }
    Ok(sum)
}
assert_eq!(two_iterator_sum(1..=10,["a","bb","ccc"])?, 61);
# // '#' needed for doctest
# Ok::<(), anyhow::Error>(())
```

Create a function that accepts an array-like thing of path-like things.
Return the number of path components at an index.

```rust
use anyinput::anyinput;
use anyhow::Result;

#[anyinput]
fn indexed_component_count(array: AnyArray<AnyPath>, index: usize) -> Result<usize, anyhow::Error> {
    // Needs .as_ref to turn the nested AnyPath into a &Path.
    let path = array[index].as_ref();
    let count = path.iter().count();
    Ok(count)
}
assert_eq!(indexed_component_count(vec!["usr/files/home","usr/data"], 1)?, 2);
# // '#' needed for doctest
# Ok::<(), anyhow::Error>(())
```

cmk todo do something interesting with 2d ndarray/views

Create a function that accepts an `NdArray`-like thing of `f32`. Return the mean.
Support for `NdArray` is provided by the optional feature `ndarray`.

```rust
use anyinput::anyinput;
#[anyinput]
fn any_mean(array: AnyNdArray<f32>) -> Result<f32, anyhow::Error> {
    let mean = array.mean().unwrap(); // cmk return error?
    Ok(mean)
}

// 'AnyNdArray' works with any 1-D array-like thing, but must be borrowed.
assert_eq!(any_mean(&[10.0, 20.0, 30.0, 40.0])?, 25.0);
assert_eq!(any_mean(&ndarray::array![10.0, 20.0, 30.0, 40.0])?, 25.0);
# // '#' needed for doctest
# Ok::<(), anyhow::Error>(())
```

AnyInputs
---------

| AnyInput   | Description                            | Creates Concrete Type           |
| ---------- | -------------------------------------- | ------------------------------- |
| AnyString  | Any string-like thing                  | `&str`                          |
| AnyPath    | Any path-like or string-like thing     | `&Path`                         |
| AnyIter    | Any iterator-like thing                | `<I as IntoIterator>::IntoIter` |
| AnyArray   | Any array-like thing                   | `&[T]`                          |
| AnyNdArray | Any 1-D array-like thing (borrow-only) | `ndarray::ArrayView1<T>`        |

Notes & Features
--------

* Works with nesting, multiple inputs, and generics.
* Automatically and efficiently converts an top-level AnyInput into a concrete type.
* An array or iterator of, for example, `AnyString` must resolve to the same string type. So, you can have a vector of all `&str` or all `String`, but not mixed. This hold true for all AnyInputs.
* When nesting, efficiently convert the nested AnyInput to the concrete type with
  *  `.as_ref()` -- AnyString, AnyPath, AnyArray
  *  `.into_iter()` -- AnyIter
  *  `.into()` -- AnyNdArray
* Easily apply `NdArray` functions to regular Rust arrays, slices, and `Vec`s.

cmk give example of including NdArray feature.

How It Works
--------

The `#[anyinput]` macro uses standard Rust generics to support multiple input types. To do this, it
 rewrites your function with the appropriate generics. It also adds lines to your function to efficiently convert from the generic to the concrete type. For example, it transforms `len_plus_2` from:

```rust
use anyinput::anyinput;

#[anyinput]
fn len_plus_2(s: AnyString) -> Result<usize, anyhow::Error> {
    Ok(s.len()+2)
}
```
into
```rust
fn len_plus_2<AnyString0: AsRef<str>>(s: AnyString0) -> Result<usize, anyhow::Error> {
    let s = s.as_ref();
    Ok(s.len() + 2)
}
```
Here `AnyString0` is the generic type. The `as_ref()` line converts the generic to the concrete type.

As with all Rust generics, the compiler creates a separate function for each combination of concrete types used by the calling code.


Project Links cmk update
-----

* [**Installation**](https://crates.io/crates/bed-reader)
* [**Documentation**](https://docs.rs/bed-reader/)
* [**Questions via email**](mailto://fastlmm-dev@python.org)
* [**Source code**](https://github.com/fastlmm/bed-reader)
* [**Discussion**](https://github.com/fastlmm/bed-reader/discussions/)
* [**Bug Reports**](https://github.com/fastlmm/bed-reader/issues)
* [**Project Website**](https://fastlmm.github.io/)

