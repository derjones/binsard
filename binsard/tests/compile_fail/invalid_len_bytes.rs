use binsard::{Decode, Encode};

#[derive(Encode, Decode)]
struct InvalidLenBytes {
    #[binsard(len_bytes = 3)]
    name: String,
}

fn main() {}

