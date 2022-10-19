#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

/// A macro for easier writing of functions that accept any string-, path-, iterator-, array-, or ndarray-like input.
/// The AnyInputs are `AnyString`, `AnyPath`, `AnyIter`, `AnyArray`, and (optionally) `AnyNdArray`.
///
/// See the [documentation](https://docs.rs/anyinput/) for for details.
///
/// # Example
/// ```
/// use anyinput::anyinput;
///
/// #[anyinput]
/// fn len_plus_2(s: AnyString) -> usize {
///     s.len()+2
/// }
///
/// // By using AnyString, len_plus_2 works with
/// // &str, String, or &String -- borrowed or moved.
/// assert_eq!(len_plus_2("Hello"), 7); // move a &str
/// let input: &str = "Hello";
/// assert_eq!(len_plus_2(&input), 7); // borrow a &str
/// let input: String = "Hello".to_string();
/// assert_eq!(len_plus_2(&input), 7); // borrow a String
/// let input2: &String = &input;
/// assert_eq!(len_plus_2(&input2), 7); // borrow a &String
/// assert_eq!(len_plus_2(input2), 7); // move a &String
/// assert_eq!(len_plus_2(input), 7); // move a String
/// ```
pub use anyinput_derive::anyinput;
