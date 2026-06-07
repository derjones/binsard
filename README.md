# binlite

A compact binary serialization format for Rust with bit-packing support.

## Crates

| Crate | Description |
|---|---|
| [`binsard`](binsard/README.md) | Main library — derive `Encode`/`Decode` for your types |
| [`binsard_derive`](binsard_derive/README.md) | Procedural macros (used automatically via `binsard`) |

## Format

The binary format is documented in detail in [FORMAT.md](FORMAT.md). It covers the two-region layout (data region + bit-packed region), type encoding rules, bit-packing behavior, and worked examples.

## Quick Start

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
let bytes = reading.encode();                    // [1, 187, 137] — just 3 bytes
let decoded = SensorReading::decode(&bytes);
assert_eq!(reading, decoded);
```
