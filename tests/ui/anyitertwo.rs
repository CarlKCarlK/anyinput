use anyinput::anyinput;

#[anyinput]
pub fn any_str_len(s: AnyIter<AnyString, usize>) -> Result<usize, anyhow::Error> {
    let len = s.len();
    Ok(len)
}

fn main() {}
