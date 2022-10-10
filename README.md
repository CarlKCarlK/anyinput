anyinput
==========

[<img alt="github" src="https://img.shields.io/badge/github-anyinput-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/CarlKCarlK/anyinput)
[<img alt="crates.io" src="https://img.shields.io/crates/v/anyinput.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/anyinput)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-anyinput-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/anyinput)
[![CI](https://github.com/CarlKCarlK/anyinput/actions/workflows/ci.yml/badge.svg)](https://github.com/CarlKCarlK/anyinput/actions/workflows/ci.yml)

A macro for easier writing of functions that accept any string-, path-, iterator-, array-, or ndarray-like input.

Do you know how to write a function that accepts any kind of string as input? (Are you sure it accepts, for example, a borrowed reference to `String`?) Do you know how to accept any kind of iterator? How about an iterator of any kind of path? How about a function that accepts a `Vec<f32>` as a `ndarray::ArrayView1`? If yes, you don't need this crate.

Rust functions can accept all these inputs, but the syntax can be hard to remember and read.
This crate provides syntactic sugar that makes writing and reading such functions easier.

The AnyInputs are `AnyString`, `AnyPath`, `AnyIter`, `AnyArray`, and (optionally) `AnyNdArray`. They may be nested.

##### Contents
  - [Usage](#usage)
  - [Examples](#examples)
  - [The AnyInputs](#the-anyinputs)
  - [Notes & Features](#notes--features)
  - [How It Works](#how-it-works)
  - [Project Links](#project-links)


Usage
-----

Add this to your `Cargo.toml`:

```toml
[dependencies]
anyinput = { version = "0.1.4", features = ["ndarray"] }
```

If you don't need `NdArray` support, omit the `ndarray` feature.

Examples
--------

We'll start with examples that are so simple that you may not need the macro.
We want to show that simple examples stay simple.

Create a function that adds `2` to the length of any string-like thing.

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
# // '# OK...' needed for doctest
# Ok::<(), anyhow::Error>(())
```

Another simple example: Create a function that counts the components of any path-like thing.

```rust
use anyinput::anyinput;
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
assert_eq!(component_count(&path)?, 3);
let pathbuf = path.to_path_buf();
assert_eq!(component_count(pathbuf)?, 3);
# // '# OK...' needed for doctest
# Ok::<(), anyhow::Error>(())
```

As we add nesting and multiple inputs, the macro becomes more useful.
Here we create a function with two inputs. One input accepts any iterator-like
thing of `usize`. The second input accepts any iterator-like thing of string-like things. The function returns the sum of the numbers and string lengths.

We apply the function to the range `1..=10` and a slice of `&str`'s.

```rust
use anyinput::anyinput;
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
# // '# OK...' needed for doctest
# Ok::<(), anyhow::Error>(())
```

Create a function that accepts an array-like thing of path-like things.
Return the number of path components at an index.

```rust
use anyinput::anyinput;
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
# // '# OK...' needed for doctest
# Ok::<(), anyhow::Error>(())
```

You can easily apply `NdArray` functions to any array-like thing of numbers. For example, 
here we create  a function that accepts an `NdArray`-like thing of `f32` and returns the mean.
We apply the function to both a `Vec` and an `Array1<f32>`.

Support for `NdArray` is provided by the optional feature `ndarray`.
```rust
use anyinput::anyinput;
use anyhow::Result;

# // '#[cfg...' needed for doctest
# #[cfg(feature = "ndarray")]
#[anyinput]
fn any_mean(array: AnyNdArray<f32>) -> Result<f32, anyhow::Error> {
    if let Some(mean) = array.mean() {
        Ok(mean)
    } else {
        Err(anyhow::anyhow!("empty array"))
    }
}

// 'AnyNdArray' works with any 1-D array-like thing, but must be borrowed.
# #[cfg(feature = "ndarray")]
assert_eq!(any_mean(&vec![10.0, 20.0, 30.0, 40.0])?, 25.0);
# #[cfg(feature = "ndarray")]
assert_eq!(any_mean(&ndarray::array![10.0, 20.0, 30.0, 40.0])?, 25.0);
# // '# OK...' needed for doctest
# Ok::<(), anyhow::Error>(())
```

The AnyInputs
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

* Feature requests and contributions are welcome.
* Works with nesting, multiple inputs, and generics.
* Automatically and efficiently converts an top-level AnyInput into a concrete type.
* Elements of AnyArray, AnyIter, and AnyNdArray must be a single type. So, `AnyArray<AnyString>` 
  accepts a vector of all `&str` or all `String`, but not mixed.
* When nesting, efficiently convert the nested AnyInput to the concrete type with
  *  `.as_ref()` -- AnyString, AnyPath, AnyArray
  *  `.into_iter()` -- AnyIter
  *  `.into()` -- AnyNdArray

  (The iterator and array examples above show this.)
* Let's you easily apply `NdArray` functions to regular Rust arrays, slices, and `Vec`s.
* Used by the [bed-reader](https://docs.rs/bed-reader/latest/bed_reader/) genomics crate.

How It Works
--------

The `#[anyinput]` macro uses standard Rust generics to support multiple input types. To do this, it
 rewrites your function with the appropriate generics. It also adds lines to your function to efficiently convert from any top-level generic to a concrete type. For example, the macro transforms `len_plus_2` from:

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
Here `AnyString0` is the generic type. The line `let s = s.as_ref()` converts from generic type `AnyString0` to concrete type `&str`.

As with all Rust generics, the compiler creates a separate function for each combination of concrete types used by the calling code.


Project Links
-----

* [**Installation**](https://crates.io/crates/anyinput)
* [**Documentation**](https://docs.rs/anyinput/)
* [**Source code**](https://github.com/CarlKCarlK/anyinput)
* [**Discussion**](https://github.com/CarlKCarlK/anyinput/discussions/)
* [**Bug Reports and Feature Requests**](https://github.com/CarlKCarlK/anyinput/issues)
