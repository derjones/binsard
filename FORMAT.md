# binsard Binary Format Specification

This document describes the compact binary format produced by `binsard` when serializing Rust structs and enums.

## Core Principle: Two Regions

Every encoded message consists of **two consecutive regions** in the output byte array:

```
┌────────────────────────────┬──────────────────────────────┐
│       Data Region          │     Bit-Packed Region         │
│   (full bytes, front)      │  (packed bits, back)         │
└────────────────────────────┴──────────────────────────────┘
        ──────────►                    ◄──────────
      read left to right           read right to left
        (ascending)                  (descending)
```

- **Data Region** (front): Contains fields encoded as whole bytes (e.g. integers without a `bits` attribute, strings, vectors, arrays, nested types). Read front to back.
- **Bit-Packed Region** (back): Contains all bit-packed fields (booleans, `Option` discriminants, fields with `#[binsard(bits = N)]`, enum tags). Read **backwards from the end of the buffer**.

### Why Backwards?

The `EncodeHelper` writes bits into an internal byte array in LSB-first order. When finalizing (`finish()`), this array is **reversed** before being appended to the data region. This allows the decoder to read the bit-packed region from the end of the overall byte buffer without needing to know the length of the data region.

---

## Type Encoding in Detail

### Primitive Integers (without `bits` attribute)

| Type | Bytes | Byte Order |
|---|---|---|
| `u8` / `i8` | 1 | — |
| `u16` / `i16` | 2 | Big-Endian |
| `u32` / `i32` / `f32` | 4 | Big-Endian |
| `u64` / `i64` / `f64` | 8 | Big-Endian |
| `u128` / `i128` | 16 | Big-Endian |

Bytes are written directly into the **data region** via `to_be_bytes()`.

**Example:** `u32` with value `42`

```
Data Region: [0x00, 0x00, 0x00, 0x2A]
```

### Primitive Integers (with `#[binsard(bits = N)]`)

Instead of being written to the data region, the lowest **N bits** of the value are written into the **bit-packed region**. Bits are packed in LSB-first order.

**Example:** `u8` with value `9` and `bits = 4`

```
Binary: 9 = 0b1001
Written bits (LSB first): 1, 0, 0, 1
```

### `bool`

Booleans are **always** written as a single bit into the bit-packed region:
- `true` → bit `1`
- `false` → bit `0`

### `String`

```
┌──────────────┬─────────────────────────┐
│ Length (N B) │    UTF-8 Bytes          │
└──────────────┴─────────────────────────┘
```

- **Default**: 1-byte length prefix (`len as u8`, max. 255 bytes)
- With `#[binsard(len_bytes = N)]` the width of the length prefix can be set to 1, 2, or 4 bytes (`u8`, `u16`, `u32`, each Big-Endian)
- Followed by the raw UTF-8 bytes

Written into the **data region**.

### `Vec<u8>`

```
┌──────────────────┬─────────────────────────┐
│ Length (N B, BE) │    Raw Bytes            │
└──────────────────┴─────────────────────────┘
```

- **Default**: 2-byte length prefix (`len as u16`, Big-Endian, max. 65535 bytes)
- With `#[binsard(len_bytes = N)]` the width of the length prefix can be set to 1, 2, or 4 bytes (`u8`, `u16`, `u32`, each Big-Endian)
- Followed by the raw bytes

Written into the **data region**.

### `Vec<T>` (T ≠ u8)

```
┌──────────────┬─────────────┬─────────────┬─────┐
│ Length (N B) │  Element 0  │  Element 1  │ ... │
└──────────────┴─────────────┴─────────────┴─────┘
```

- **Default**: 1-byte length prefix (`len as u8`, max. 255 elements)
- With `#[binsard(len_bytes = N)]` the width of the length prefix can be set to 1, 2, or 4 bytes (`u8`, `u16`, `u32`, each Big-Endian)
- Followed by the recursively encoded elements

The data portions of elements go into the **data region**; the bit portions flow into the **shared bit-packed area** (same `EncodeHelper`).

### `[u8; N]` (Byte Arrays)

Bytes are copied directly and without a length prefix into the **data region**, since the size N is known at compile time.

### `Option<T>`

```
Bit-Packed Region:  ┌── 1-bit discriminant ──┐
                    │  1 = Some, 0 = None     │
                    └─────────────────────────┘
```

- 1 bit in the bit-packed region: `1` for `Some`, `0` for `None`
- For `Some`: the inner value `T` is then encoded normally (into data or bit-packed region depending on its type)
- For `None`: no further data

### Nested Structs

Nested structs are encoded via `encode_internal(helper)`. The **same `EncodeHelper`** is passed through from the outer struct:

- The data portion of the inner struct is inserted into the data region
- Bit-packed fields of the inner struct flow into the **same** bit stream as the outer struct

### Enums

Enums consist of a **tag** (variant index) and the fields of the variant.

```
Bit-Packed Region:  ┌── Tag (N bits) ──┬── Variant Fields ──┐
                    │  Variant Index    │   (by type)        │
                    └───────────────────┴────────────────────┘
```

- **Tag width**: Default 8 bits. Can be configured with `#[binsard(bits = N)]` at the type level.
- **Tag value**: 0-based index of the variant in the enum definition.
- After the tag, the fields of the respective variant follow (encoded normally).

**Example:**

```rust
#[binsard(bits = 2)]
enum Priority { Low, Medium, High }
```

| Variant | Tag (2 bits) |
|---|---|
| `Low` | `00` |
| `Medium` | `01` |
| `High` | `10` |

---

## Bit-Packing in Detail

### Write Order

All bit-packed fields are written in **field order** (as defined in the struct/enum) into a shared bit stream. Within this stream:

1. **LSB-first** per field: The least significant bit is written first.
2. **Byte packing**: Bits are written from position 0 (LSB) to position 7 (MSB) into each byte. When a byte is full, a new one is started.

### Byte Layout Before Reverse

The internal `EncodeHelper` buffer builds bytes as follows:

```
Byte 0:  [bit0, bit1, bit2, bit3, bit4, bit5, bit6, bit7]
Byte 1:  [bit8, bit9, bit10, ...]
...
```

Where `bit0` is the first written bit (LSB of the first field).

### Reverse on Finalization

When `finish()` is called, the internal byte array is **reversed**:

```
Before reverse: [Byte_0, Byte_1, ..., Byte_N]
After reverse:  [Byte_N, ..., Byte_1, Byte_0]
```

This reversed array is appended to the data region.

### Reading During Decoding

The `DecodeHelper` reads bits **from the end of the overall byte buffer**:

```rust
let byte_idx = data.len() - (bit_idx / 8) - 1;
let off = bit_idx % 8;
```

- `bit_idx = 0` reads the last byte, bit position 0 → this was the first written bit
- `bit_idx = 1` reads the last byte, bit position 1 → the second written bit
- etc.

This reproduces the exact write order during reading.

---

## `reserve_bits`

The attribute `#[binsard(reserve_bits = N)]` reserves N bits at the **beginning** of the bit stream (with value 0). Both encoder and decoder skip these bits. This is useful for:

- Versioning fields
- Padding/alignment
- Future extensions

```
Bit stream:  [N reserved bits (0)] [actual fields...]
```

---

## Complete Example: Step by Step

### Struct Definition

```rust
#[derive(Encode, Decode)]
struct SensorReading {
    #[binsard(bits = 4)]
    channel: u8,        // 4 bits → bit-packed
    #[binsard(bits = 12)]
    value: u16,         // 12 bits → bit-packed
    active: bool,       // 1 bit → bit-packed
}
```

### Encoding `SensorReading { channel: 9, value: 3000, active: true }`

**Step 1: Data Region**

No field is written to the data region (all are bit-packed).

```
Data Region: [] (empty)
```

**Step 2: Build Bit-Packed Region**

| Field | Value | Bits | Binary (LSB first) |
|---|---|---|---|
| `channel` | 9 | 4 | `1, 0, 0, 1` |
| `value` | 3000 | 12 | `0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1` |
| `active` | true | 1 | `1` |

Total: 4 + 12 + 1 = **17 bits** → **3 bytes** (rounded up)

**Internal byte array (before reverse):**

```
Byte 0 (bits 0-7):
  Bit 0: channel[0] = 1  ─┐
  Bit 1: channel[1] = 0   │ channel = 9 = 0b1001
  Bit 2: channel[2] = 0   │
  Bit 3: channel[3] = 1  ─┘
  Bit 4: value[0]   = 0  ─┐
  Bit 5: value[1]   = 0   │
  Bit 6: value[2]   = 0   │ value bits 0-3
  Bit 7: value[3]   = 1  ─┘

  → Byte 0 = 0b1000_1001 = 0x89

Byte 1 (bits 8-15):
  Bit 0: value[4]   = 1  ─┐
  Bit 1: value[5]   = 1   │
  Bit 2: value[6]   = 1   │
  Bit 3: value[7]   = 0   │ value bits 4-11
  Bit 4: value[8]   = 1   │
  Bit 5: value[9]   = 1   │
  Bit 6: value[10]  = 1   │
  Bit 7: value[11]  = 0  ─┘

  → Byte 1 = 0b0111_0111... Wait:

  Correct: 3000 = 0xBB8 = 0b1011_1011_1000
  LSB-first: Bit 0=0, 1=0, 2=0, 3=1, 4=1, 5=1, 6=0, 7=1, 8=1, 9=1, 10=0, 11=1

  Bits 4-7 of value (= bits 4-7 of the stream on byte 0):
  value[0]=0, value[1]=0, value[2]=0, value[3]=1
  → upper 4 bits of byte 0: 1000 → 0x80
  → Byte 0 = 0b1000_1001 = 0x89 ✓

  Bits 8-15 (= value[4..11], all on byte 1):
  value[4]=1, value[5]=1, value[6]=0, value[7]=1, value[8]=1, value[9]=1, value[10]=0, value[11]=1
  → Byte 1 = 0b1011_0110... No:
  Position in byte: off 0=value[4]=1, off 1=value[5]=1, off 2=value[6]=0, off 3=value[7]=1,
                    off 4=value[8]=1, off 5=value[9]=1, off 6=value[10]=0, off 7=value[11]=1
  → Byte 1 = 0b_1_0_1_1_1_0_1_1 = 0xBB ✓

Byte 2 (bit 16):
  Bit 0: active = 1
  → Byte 2 = 0b0000_0001 = 0x01
```

**Step 3: Reverse**

```
Before reverse: [0x89, 0xBB, 0x01]
After reverse:  [0x01, 0xBB, 0x89]
```

**Step 4: Concatenate**

```
Data Region (empty) + Bit-Packed Region (reversed) = [0x01, 0xBB, 0x89]
                                                    = [1, 187, 137]
```

Result: **3 bytes** instead of 4 bytes (u8 + u16 + bool uncompressed).

---

## Complex Example: Mixed Fields

```rust
#[derive(Encode, Decode)]
struct CompactFlags {
    flag_a: bool,            // 1 bit → bit-packed
    flag_b: bool,            // 1 bit → bit-packed
    flag_c: bool,            // 1 bit → bit-packed
    #[binsard(bits = 3)]
    priority: u8,            // 3 bits → bit-packed
    #[binsard(bits = 5)]
    seq_num: u8,             // 5 bits → bit-packed
    payload: Vec<u8>,        // data region
}
```

### Encoding `CompactFlags { flag_a: true, flag_b: false, flag_c: true, priority: 7, seq_num: 31, payload: vec![0xAA; 3] }`

**Data Region:**

```
┌───────────────────┬──────────────────────┐
│ Vec<u8> Length    │ Vec<u8> Content      │
│ 0x00, 0x03 (BE)  │ 0xAA, 0xAA, 0xAA    │
└───────────────────┴──────────────────────┘
= [0x00, 0x03, 0xAA, 0xAA, 0xAA]  (5 bytes)
```

**Bit-Packed Region:**

| Field | Value | Bits | Binary (LSB first) |
|---|---|---|---|
| `flag_a` | true | 1 | `1` |
| `flag_b` | false | 1 | `0` |
| `flag_c` | true | 1 | `1` |
| `priority` | 7 | 3 | `1, 1, 1` |
| `seq_num` | 31 | 5 | `1, 1, 1, 1, 1` |

Total: 1+1+1+3+5 = **11 bits** → **2 bytes**

```
Byte 0 (bits 0-7):  flag_a=1, flag_b=0, flag_c=1, priority=111, seq_num[0..1]=11
                     = 0b11_111_1_0_1 = 0b1111_1101 = 0xFD

Byte 1 (bits 8-10): seq_num[2..4]=111
                     = 0b0000_0111 = 0x07
```

Reversed: `[0x07, 0xFD]`

**Total output:**

```
[0x00, 0x03, 0xAA, 0xAA, 0xAA, 0x07, 0xFD]
 ├──────── Data Region ─────────┤├ Bit-Packed ┤
           (5 bytes)               (2 bytes)
```

---

## Enum Example with Nested Types

```rust
#[derive(Encode, Decode)]
#[binsard(bits = 2)]
enum SmallEnum { Off, Low, High }

#[derive(Encode, Decode)]
#[binsard(bits = 4)]
enum SensorMessage {
    Reset,                                // Tag = 0
    Reading(SensorReading),               // Tag = 1
    Batch { count: u8, active: bool },    // Tag = 2
}

#[derive(Encode, Decode)]
struct Packet {
    #[binsard(bits = 4)]
    version: u8,
    mode: SmallEnum,
    msg: SensorMessage,
    #[binsard(bits = 13)]
    tracker_version: Option<u16>,
}
```

### Bit Stream Order for `Packet`

The bit stream is built in field order:

```
┌─────────┬──────────┬───────────────────────┬───────────────────────────────┐
│ version │ SmallEnum│    SensorMessage       │     Option<u16>              │
│ 4 bits  │  Tag     │  Tag + Fields         │ 1-bit discr. + up to 13 bits │
│         │  2 bits  │  4 bits + ...         │                              │
└─────────┴──────────┴───────────────────────┴───────────────────────────────┘
```

The nested enums (`SmallEnum`, `SensorMessage`) write their tag and fields into the **same** bit stream via the passed-through `EncodeHelper`.

Fields that are not bit-packed (e.g. `count: u8` without a `bits` attribute in `Batch`) are written to the data region.

---

## Summary of Encoding Rules

| Field Type | Region | Encoding |
|---|---|---|
| `bool` | Bit-Packed | 1 bit |
| Integer without `bits` | Data | Big-Endian, full byte size |
| Integer with `bits = N` | Bit-Packed | N bits, LSB-first |
| `String` | Data | 1 B length + UTF-8 (configurable with `len_bytes`) |
| `Vec<u8>` | Data | 2 B length (BE) + raw bytes (configurable with `len_bytes`) |
| `Vec<T>` | Data | 1 B length + recursively encoded elements (configurable with `len_bytes`) |
| `[u8; N]` | Data | N raw bytes |
| `Option<T>` | Bit-Packed + depends on T | 1-bit discriminant + value for `Some` |
| Enum tag | Bit-Packed | N bits (default 8, configurable) |
| Nested structs/enums | Both | Shared bit stream, data inline |

### Key Properties

- **No header/magic bytes**: The format has no general header. Structure is derived from the type.
- **Not self-describing**: Encoder and decoder must use the same type with identical attributes.
- **Deterministic size**: For types without variable-length fields (`String`, `Vec`), the output size is computable at compile time.
- **Endianness**: All multi-byte integers are written in **Big-Endian**. Bit-packed fields are written in **LSB-first** order within bytes.
- **Shared bit stream**: Nested types share the same `EncodeHelper`, so bits are efficiently packed across type boundaries.
