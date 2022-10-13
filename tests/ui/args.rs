use anyinput::anyinput;

#[anyinput(not_empty)]
pub fn any_str_len(s: AnyIter(AnyString)) -> Result<usize, anyhow::Error> {
    let len = s.len();
    Ok(len)
}

fn main() {}
