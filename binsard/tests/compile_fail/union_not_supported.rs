use binsard::{Decode, Encode};

union MyUnion {
    a: u32,
    b: u64,
}

#[derive(Encode, Decode)]
struct WrapUnion {
    u: MyUnion,
}

fn main() {}

