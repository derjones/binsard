use binsard::{Decode, DecodeError, Encode};

#[derive(Encode, Decode, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    // Successful decode
    let p = Point { x: 10, y: -5 };
    let bytes = p.encode();
    match Point::decode(&bytes) {
        Ok(decoded) => println!("Decoded: {decoded:?}"),
        Err(e) => println!("Error: {e}"),
    }

    // Truncated input
    match Point::decode(&[0x00, 0x01]) {
        Ok(_) => println!("Should not reach here"),
        Err(DecodeError::UnexpectedEof) => println!("Caught truncated input (expected)"),
        Err(e) => println!("Unexpected error: {e}"),
    }

    // Empty input
    match Point::decode(&[]) {
        Ok(_) => println!("Should not reach here"),
        Err(DecodeError::UnexpectedEof) => println!("Caught empty input (expected)"),
        Err(e) => println!("Unexpected error: {e}"),
    }
}
