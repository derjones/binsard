# binsard_derive

Procedural derive macros for the [binsard](https://crates.io/crates/binsard) binary serialization library.

This crate provides `#[derive(Encode)]` and `#[derive(Decode)]` macros that generate compact binary serialization code with bit-packing support.

**You should not depend on this crate directly.** Use `binsard` instead, which re-exports the derive macros automatically.

## Usage

```toml
[dependencies]
binsard = "0.1"
```

```rust
use binsard::{Encode, Decode};

#[derive(Encode, Decode)]
struct Message {
    id: u32,
    active: bool,
    payload: Vec<u8>,
}
```

## Attributes

### Type-level: `#[binsard(bits = N)]`

Controls the bit-width of enum discriminant tags. Default is 8 (one full byte).

```rust
#[derive(Encode, Decode)]
#[binsard(bits = 2)]
enum Priority { Low, Medium, High }
```

### Field-level: `#[binsard(bits = N)]`

Packs an integer field into exactly N bits instead of its full byte width.

```rust
#[derive(Encode, Decode)]
struct Sensor {
    #[binsard(bits = 4)]
    channel: u8,
    #[binsard(bits = 12)]
    value: u16,
}
```

### Field-level: `#[binsard(len_bytes = N)]`

Overrides the byte width of the length prefix for `String`, `Vec<u8>`, and `Vec<T>` fields. Allowed values are 1, 2, or 4 (corresponding to `u8`, `u16`, `u32`).

```rust
#[derive(Encode, Decode)]
struct LargeMessage {
    #[binsard(len_bytes = 2)]
    name: String,         // u16 prefix instead of default u8
    #[binsard(len_bytes = 4)]
    data: Vec<u8>,        // u32 prefix instead of default u16
}
```

See the [binsard documentation](https://crates.io/crates/binsard) for full details.

## License

MIT
