#[cfg(test)]
mod tests {
    use input_like_derive::input_special;

    #[test]
    fn one_ndarray_usize_input() {
        #[input_special]
        pub fn any_slice_len(a: NdArrayLike<usize>) -> Result<usize, anyhow::Error> {
            let len = a.len();
            Ok(len)
        }
        assert_eq!(any_slice_len([1, 2, 3].as_ref()).unwrap(), 3);
    }
}
