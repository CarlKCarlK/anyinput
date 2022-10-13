use anyinput::anyinput;

#[anyinput]
fn full_string(s: AnyString<AnyPath>) {
    println!("{}", s);
}

fn main() {}
