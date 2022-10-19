use anyinput::anyinput;

#[anyinput]
pub fn any_str_len(s: AnyIter(AnyString)) -> usize {
    s.len()
}

fn main() {}
