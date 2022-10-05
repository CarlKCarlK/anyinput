anyinput
==========

cmk[<img alt="github" src="https://img.shields.io/badge/github-bed--reader-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/fastlmm/bed-reader)
[<img alt="crates.io" src="https://img.shields.io/crates/v/bed-reader.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/bed-reader)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-bed--reader-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/bed-reader)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/fastlmm/bed-reader/CI/master?style=for-the-badge" height="20">](https://github.com/fastlmm/bed-reader/actions?query=branch%3Amaster)

Create functions that accept any type of string, path, iterator-like, or array-line inputs.

Examples
--------

cmk nested, multiple, works with other generics, array of string-like requires req.
cmk two AnyString inputs don't have to be the same type.
cmk any array of AnyString must be the same string type.
cmk how it works

Create a function that adds 2 to the length of any string-like thing.

```rust
use anyinput_derive::anyinput; //cmk need passthru
use anyhow::Result;

#[anyinput]
fn len_plus_2(s: AnyString) -> Result<usize, anyhow::Error> {
    Ok(s.len()+2)
}
assert_eq!(len_plus_2("hello")?, 7);
let input: &str = "Hello";
assert_eq!(len_plus_2(&input)?, 7); // borrow a &str
assert_eq!(len_plus_2(input)?, 7); // move a &str
let input: String = "Hello".to_string();
assert_eq!(len_plus_2(&input)?, 7); // borrow a String
let input2: &String = &input;
assert_eq!(len_plus_2(&input2)?, 7); // borrow a &String
assert_eq!(len_plus_2(input2)?, 7); // move a &String
assert_eq!(len_plus_2(input)?, 7); // move a String
# Ok::<(), anyhow::Error>(())
```

cmk format this code

Create a function that counts the components of any path-like thing.

```rust
use anyinput_derive::anyinput; //cmk need passthru
use anyhow::Result;
use std::path::{PathBuf,Path};

#[anyinput]
fn component_count(path: AnyPath) -> Result<usize, anyhow::Error> {
    let count = path.iter().count();
    Ok(count)
}

// 'AnyPath' works on any string-like or path-like thing, borrowed or moved.
assert_eq!(component_count("usr/files/home")?, 3);
let path = Path::new("usr/files/home");
assert_eq!(component_count(&path)?, 3);
assert_eq!(component_count(path.to_path_buf())?, 3);

# Ok::<(), anyhow::Error>(())
```

Create a function that accepts an any iterator-like thing of `usize`
and any iterator-like thing of string-like things and returns a sum.

Nesting AnyInput is allowed.

```rust
use anyinput_derive::anyinput; //cmk need passthru
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
# Ok::<(), anyhow::Error>(())
```

Create a function that accepts an array-like list of path-like things.
Return the number of path components at an index.

```rust
use anyinput_derive::anyinput; //cmk need passthru
use anyhow::Result;

#[anyinput]
fn indexed_component_count(array: AnyArray<AnyPath>, index: usize) -> Result<usize, anyhow::Error> {
    // Needs .as_ref to turn the nested AnyPath into a Path.
    let path = array[index].as_ref();
    let count = path.iter().count();
    Ok(count)
}
assert_eq!(indexed_component_count(vec!["usr/files/home","usr/data"], 1)?, 2);
# Ok::<(), anyhow::Error>(())
```

Read every second individual (samples) and SNPs (variants) 20 to 30.

```ignore
# // '#' needed for doctest
# use bed_reader::{Bed, ReadOptions, assert_eq_nan, sample_bed_file};
use ndarray::s;

let file_name = sample_bed_file("some_missing.bed")?;
let mut bed = Bed::new(file_name)?;
let val = ReadOptions::builder()
    .iid_index(s![..;2])
    .sid_index(20..30)
    .f64()
    .read(&mut bed)?;

assert!(val.dim() == (50, 10));
# use bed_reader::BedErrorPlus; // '#' needed for doctest
# Ok::<(), BedErrorPlus>(())
```

List the first 5 individual (sample) ids, the first 5 SNP (variant) ids,
and every unique chromosome. Then, read every genomic value in chromosome 5.

```ignore
# use ndarray::s; // '#' needed for doctest
# use bed_reader::{Bed, ReadOptions, assert_eq_nan, sample_bed_file};
# let file_name = sample_bed_file("some_missing.bed")?;
use std::collections::HashSet;

let mut bed = Bed::new(file_name)?;
println!("{:?}", bed.iid()?.slice(s![..5])); // Outputs ndarray: ["iid_0", "iid_1", "iid_2", "iid_3", "iid_4"]
println!("{:?}", bed.sid()?.slice(s![..5])); // Outputs ndarray: ["sid_0", "sid_1", "sid_2", "sid_3", "sid_4"]
println!("{:?}", bed.chromosome()?.iter().collect::<HashSet<_>>());
// Outputs: {"12", "10", "4", "8", "19", "21", "9", "15", "6", "16", "13", "7", "17", "18", "1", "22", "11", "2", "20", "3", "5", "14"}
let val = ReadOptions::builder()
    .sid_index(bed.chromosome()?.map(|elem| elem == "5"))
    .f64()
    .read(&mut bed)?;

assert!(val.dim() == (100, 6));
# use bed_reader::BedErrorPlus; // '#' needed for doctest
# Ok::<(), BedErrorPlus>(())
```

Project Links
-----

* [**Installation**](https://crates.io/crates/bed-reader)
* [**Documentation**](https://docs.rs/bed-reader/)
* [**Questions via email**](mailto://fastlmm-dev@python.org)
* [**Source code**](https://github.com/fastlmm/bed-reader)
* [**Discussion**](https://github.com/fastlmm/bed-reader/discussions/)
* [**Bug Reports**](https://github.com/fastlmm/bed-reader/issues)
* [**Project Website**](https://fastlmm.github.io/)

