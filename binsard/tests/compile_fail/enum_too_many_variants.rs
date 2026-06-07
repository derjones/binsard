use binsard::{Encode, Decode};

#[derive(Encode, Decode)]
#[binsard(bits = 1)]
enum TooMany {
    A,
    B,
    C,
}

fn main() {}
