use binsard::{Decode, Encode};

#[derive(Encode, Decode)]
struct UnsupportedOptionInner {
    value: Option<[u8; 1 + 1]>,
}

fn main() {}

