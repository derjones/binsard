use binsard::{Decode, Encode};

#[derive(Encode, Decode, Debug, PartialEq)]
struct Message {
    id: u32,
    active: bool,
    payload: Vec<u8>,
}

fn main() {
    let msg = Message {
        id: 42,
        active: true,
        payload: vec![0xDE, 0xAD],
    };

    let bytes = msg.encode();
    println!("Encoded: {bytes:?} ({} bytes)", bytes.len());

    let decoded = Message::decode(&bytes).unwrap();
    println!("Decoded: {decoded:?}");
    assert_eq!(msg, decoded);
    println!("Roundtrip OK!");
}
