use anyinput::anyinput;

#[anyinput]
fn empty_array(array: AnyArray) {
    println!(array[0]);
}

fn main() {}
