use binsard::{Decode, Encode};

#[derive(Encode, Decode, Debug, PartialEq)]
struct SensorReading {
    #[binsard(bits = 4)]
    channel: u8,
    #[binsard(bits = 12)]
    value: u16,
    active: bool,
}

#[derive(Encode, Decode, Debug, PartialEq)]
#[binsard(bits = 2)]
enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Encode, Decode, Debug, PartialEq)]
struct Packet {
    #[binsard(bits = 4)]
    version: u8,
    priority: Priority,
    sensor: SensorReading,
}

fn main() {
    let packet = Packet {
        version: 3,
        priority: Priority::High,
        sensor: SensorReading {
            channel: 12,
            value: 2048,
            active: true,
        },
    };

    let bytes = packet.encode();
    println!("Packet encoded: {bytes:?} ({} bytes)", bytes.len());
    println!("  version(4b) + priority(2b) + channel(4b) + value(12b) + active(1b) = 23 bits");

    let decoded = Packet::decode(&bytes).unwrap();
    assert_eq!(packet, decoded);
    println!("Decoded: {decoded:?}");
}
