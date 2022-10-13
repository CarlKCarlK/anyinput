use anyinput::anyinput;

#[anyinput]
pub fn any_str_len(s: AnyIter<3>) -> Result<usize, anyhow::Error> {
    let len = s.len();
    Ok(len)
}

fn main() {}
