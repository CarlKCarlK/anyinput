// use input_like::input_like;
// use std::path::{Path, PathBuf};

// #[input_like]
// pub fn any_path_part_count(path: impl AsRef<Path>) -> Result<usize, anyhow::Error> {
//     let path = path.as_ref();
//     let count = path.iter().count();
//     Ok(count)
// }

// fn main() -> Result<(), anyhow::Error> {
//     let path: &str = "bed_reader/tests/data/small.bed";
//     assert!(any_path_part_count(&path)? == 4); // borrow a &str
//     assert!(any_path_part_count(path)? == 4); // move a &str
//     let path: String = "bed_reader/tests/data/small.bed".to_string();
//     assert!(any_path_part_count(&path)? == 4); // borrow a String
//     let path2: &String = &path;
//     assert!(any_path_part_count(&path2)? == 4); // borrow a &String
//     assert!(any_path_part_count(path2)? == 4); // move a &String
//     assert!(any_path_part_count(path)? == 4); // move a String
//     let path: &Path = Path::new("bed_reader/tests/data/small.bed");
//     assert!(any_path_part_count(&path)? == 4); // borrow a Path
//     assert!(any_path_part_count(path)? == 4); // move a Path
//     let path: PathBuf = PathBuf::from("bed_reader/tests/data/small.bed");
//     assert!(any_path_part_count(&path)? == 4); // borrow a PathBuf
//     let path2: &PathBuf = &path;
//     assert!(any_path_part_count(&path2)? == 4); // borrow a &PathBuf
//     assert!(any_path_part_count(path2)? == 4); // move a &PathBuf
//     assert!(any_path_part_count(path)? == 4); // move a PathBuf

//     Ok(())
// }
