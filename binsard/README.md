# binsard

Compact binary serialization with derive macros and bit-packing support.

`binsard` lets you derive `Encode` and `Decode` for your Rust structs and enums, producing a compact binary representation. Fields like booleans, enum tags, and small integers can be packed into individual bits using the `#[binsard(bits = N)]` attribute.

## Usage

```toml
[dependencies]
binsard = "0.1"
```

```rust
use binsard::{Encode, Decode};

#[derive(Encode, Decode, Debug, PartialEq)]
struct SensorReading {
    #[binsard(bits = 4)]
    channel: u8,
    #[binsard(bits = 12)]
    value: u16,
    active: bool,
}

let reading = SensorReading { channel: 9, value: 3000, active: true };
let bytes = reading.encode();   // [1, 187, 137] -- just 3 bytes
let decoded = SensorReading::decode(&bytes);
assert_eq!(reading, decoded);
```

## Supported types

| Type | Encoding |
|---|---|
| `bool` | 1 bit (packed into helper) |
| `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64` | Big-endian bytes, or N bits with `#[binsard(bits = N)]` |
| `String` | 1-byte length prefix + UTF-8 bytes (override with `#[binsard(len_bytes = N)]`) |
| `Vec<u8>` | 2-byte length prefix + raw bytes (override with `#[binsard(len_bytes = N)]`) |
| `Vec<T>` | 1-byte length prefix + encoded elements (override with `#[binsard(len_bytes = N)]`) |
| `[u8; N]` | Raw bytes |
| `Option<T>` | 1-bit discriminant + encoded value if `Some` |
| Nested structs/enums | Recursive encoding |

## Bit-packing

Booleans, `Option` discriminants, and fields annotated with `#[binsard(bits = N)]` are packed into a compact bit region appended to the end of the encoded output. This is especially useful for protocols and formats where every byte counts.

```rust
#[derive(Encode, Decode)]
#[binsard(bits = 2)]  // 2-bit enum tag instead of a full byte
enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Encode, Decode)]
struct Packet {
    #[binsard(bits = 4)]
    version: u8,          // 4 bits
    priority: Priority,   // 2 bits (from enum-level attribute)
    flags: bool,          // 1 bit
    payload: Vec<u8>,     // normal byte encoding
}
```

## Length prefix override

By default, `String` and `Vec<T>` use a 1-byte length prefix, and `Vec<u8>` uses a 2-byte prefix. Use `#[binsard(len_bytes = N)]` to override the prefix size (allowed values: 1, 2, or 4):

```rust
#[derive(Encode, Decode)]
struct LargePayload {
    #[binsard(len_bytes = 2)]
    name: String,         // u16 length prefix (up to 65535 bytes)
    #[binsard(len_bytes = 4)]
    data: Vec<u8>,        // u32 length prefix (up to 4 GiB)
}
```

## Struct and enum support

All struct and enum forms are supported:

```rust
#[derive(Encode, Decode)]
struct Named { x: i32, y: i32 }       // named fields

#[derive(Encode, Decode)]
struct Tuple(u32, u8);                 // tuple struct

#[derive(Encode, Decode)]
struct Unit;                           // unit struct

#[derive(Encode, Decode)]
enum Command {
    Ping,                              // unit variant
    Move { x: i32, y: i32 },          // named fields variant
    Send(Vec<u8>),                     // tuple variant
}
```

## Benchmarks

Measured with [Criterion](https://github.com/bheisler/criterion.rs) on Apple M-series silicon (`cargo bench`):

| Benchmark | Time |
|---|---|
| `sensor_encode` (2 bit-packed fields + bool) | 60.6 ns |
| `sensor_decode` | 3.7 ns |
| `packet_encode` (nested structs + enums + Option, all bit-packed) | 72.2 ns |
| `packet_decode` | 15.8 ns |
| `compact_flags_encode` (3 bools + bit fields + 64-byte Vec) | 144.3 ns |
| `compact_flags_decode` | 29.5 ns |
| `write_partly` 12-bit × 100 | 670.7 ns |
| `read_partly` 12-bit × 100 | 410.2 ns |

## License

MIT