//! Compact binary serialization with derive macros and bit-packing support.
//!
//! `binsard` lets you derive [`Encode`] and [`Decode`] for your Rust structs and enums,
//! producing a compact binary representation. Fields like booleans, enum tags, and small
//! integers can be packed into individual bits using the `#[binsard(bits = N)]` attribute.
//!
//! # Quick start
//!
//! ```
//! use binsard::{Encode, Decode};
//!
//! #[derive(Encode, Decode, Debug, PartialEq)]
//! struct Message {
//!     id: u32,
//!     active: bool,
//!     payload: Vec<u8>,
//! }
//!
//! fn main() {
//!     let msg = Message { id: 42, active: true, payload: vec![0xDE, 0xAD] };
//!     let bytes = msg.encode();
//!     let decoded = Message::decode(&bytes).unwrap();
//!     assert_eq!(msg, decoded);
//! }
//! ```
//!
//! # Bit-packing
//!
//! Use `#[binsard(bits = N)]` on integer fields to pack them into exactly N bits,
//! or on enums to control the tag width:
//!
//! ```
//! use binsard::{Encode, Decode};
//!
//! #[derive(Encode, Decode, Debug, PartialEq)]
//! struct Sensor {
//!     #[binsard(bits = 4)]
//!     channel: u8,
//!     #[binsard(bits = 12)]
//!     value: u16,
//!     active: bool,
//! }
//!
//! fn main() {
//!     let s = Sensor { channel: 9, value: 3000, active: true };
//!     let bytes = s.encode();
//!     assert_eq!(bytes.len(), 3); // 4 + 12 + 1 = 17 bits -> 3 bytes
//!     assert_eq!(s, Sensor::decode(&bytes).unwrap());
//! }
//! ```

pub use binsard_derive::Decode;
pub use binsard_derive::Encode;

/// Error type returned when decoding fails.
///
/// This covers all failure modes during binary deserialization: truncated
/// input, invalid UTF-8 in string fields, and unrecognized enum tags.
///
/// # Example
///
/// ```
/// use binsard::{Encode, Decode, DecodeError};
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// struct Pair(u32, u8);
///
/// fn main() {
///     let result = Pair::decode(&[0x00]); // too short
///     assert!(result.is_err());
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The input byte slice was shorter than expected.
    UnexpectedEof,
    /// A string field contained invalid UTF-8.
    InvalidUtf8,
    /// An enum tag did not match any known variant.
    UnknownTag(u8),
    /// A length prefix in the input is unreasonably large.
    LengthTooLarge,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in string field"),
            Self::UnknownTag(tag) => write!(f, "unknown enum tag: {tag}"),
            Self::LengthTooLarge => write!(f, "length prefix is too large"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Serialize this value into a compact binary representation.
///
/// Typically derived via `#[derive(Encode)]`. Booleans and fields annotated with
/// `#[binsard(bits = N)]` are bit-packed into a helper region appended at the end
/// of the byte output.
///
/// # Example
///
/// ```
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, PartialEq, Debug)]
/// struct Point { x: i32, y: i32 }
///
/// fn main() {
///     let p = Point { x: 10, y: -5 };
///     let bytes = p.encode();
///     assert_eq!(bytes.len(), 8); // two i32 = 8 bytes
/// }
/// ```
pub trait Encode {
    /// Encode this value into a `Vec<u8>`.
    fn encode(&self) -> Vec<u8>;
}

/// Deserialize a value from its binary representation.
///
/// Typically derived via `#[derive(Decode)]`. The input slice must have been
/// produced by the corresponding [`Encode`] implementation.
///
/// # Example
///
/// ```
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, PartialEq, Debug)]
/// struct Point { x: i32, y: i32 }
///
/// fn main() {
///     let original = Point { x: 10, y: -5 };
///     let bytes = original.encode();
///     let decoded = Point::decode(&bytes).unwrap();
///     assert_eq!(original, decoded);
/// }
/// ```
pub trait Decode: Sized {
    /// Decode a value from a byte slice.
    ///
    /// Returns a [`DecodeError`] if the input is malformed: too short,
    /// contains invalid UTF-8 in a string field, or has an unrecognized
    /// enum tag.
    fn decode(data: &[u8]) -> Result<Self, DecodeError>;
}

/// Helper for encoding bit-packed fields during serialization.
///
/// Used internally by the generated `Encode` implementations. You typically
/// do not need to use this directly unless you are implementing `Encode` manually.
///
/// Bits are written in LSB-first order within each byte. After all fields have
/// been written, call [`finish`](EncodeHelper::finish) to get the final byte
/// vector (reversed for the decoder to read from the end).
///
/// # Example
///
/// ```
/// use binsard::EncodeHelper;
///
/// let mut helper = EncodeHelper::default();
/// helper.write_bit(true);
/// helper.write_partly(42u64, 6);
/// helper.write_bit(false);
///
/// let bytes = helper.finish();
/// assert_eq!(bytes.len(), 1); // 1 + 6 + 1 = 8 bits = 1 byte
/// ```
#[derive(Default)]
pub struct EncodeHelper {
    data: Vec<u8>,
    bit_idx: usize,
}

impl EncodeHelper {
    /// Returns `true` if any bits have been written to this helper.
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::EncodeHelper;
    ///
    /// let mut helper = EncodeHelper::default();
    /// assert!(!helper.has_data());
    /// helper.write_bit(true);
    /// assert!(helper.has_data());
    /// ```
    #[inline]
    pub const fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    /// Advance the bit index by `bits` positions, pre-allocating zero bytes.
    ///
    /// Used by the generated code to reserve space for the `#[binsard(reserve_bits = N)]`
    /// attribute. Reserved bits are initialized to zero.
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::EncodeHelper;
    ///
    /// let mut helper = EncodeHelper::default();
    /// helper.reserve_bits(4);
    /// helper.write_partly(0b1010u64, 4);
    ///
    /// let bytes = helper.finish();
    /// assert_eq!(bytes.len(), 1); // 4 reserved + 4 written = 8 bits = 1 byte
    /// ```
    #[inline]
    pub fn reserve_bits(&mut self, bits: usize) {
        let end_bit = self.bit_idx + bits;
        let bytes_needed = end_bit.div_ceil(8);
        if bytes_needed > self.data.len() {
            self.data.resize(bytes_needed, 0);
        }
        self.bit_idx = end_bit;
    }

    /// Write a single bit (boolean value) to the bit stream.
    ///
    /// `true` sets the bit to 1, `false` to 0. A new zero-byte is allocated
    /// when the current byte is full.
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::EncodeHelper;
    ///
    /// let mut helper = EncodeHelper::default();
    /// helper.write_bit(true);
    /// helper.write_bit(false);
    /// helper.write_bit(true);
    ///
    /// let bytes = helper.finish();
    /// assert_eq!(bytes, vec![0b101]); // bits: 1, 0, 1
    /// ```
    #[inline]
    pub fn write_bit(&mut self, value: bool) {
        let off = self.bit_idx % 8;
        if off == 0 {
            self.data.push(0);
        }
        if value {
            let byte_idx = self.bit_idx / 8;
            self.data[byte_idx] |= 1 << off;
        }
        self.bit_idx += 1;
    }

    /// Write the lowest `bits` bits of `value` into the bit stream.
    ///
    /// Bits are written in LSB-first order. Multiple bits are packed into the
    /// current byte before spilling into the next, making this significantly
    /// faster than calling [`write_bit`](EncodeHelper::write_bit) in a loop.
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::EncodeHelper;
    ///
    /// let mut helper = EncodeHelper::default();
    /// helper.write_partly(0b1010u64, 4); // write 4 bits: value 10
    /// helper.write_partly(0b11111u64, 5); // write 5 bits: value 31
    ///
    /// let bytes = helper.finish();
    /// // 4 + 5 = 9 bits -> 2 bytes
    /// assert_eq!(bytes.len(), 2);
    /// ```
    #[inline]
    pub fn write_partly(&mut self, value: u64, bits: usize) {
        let mut written = 0;
        while written < bits {
            let off = self.bit_idx % 8;
            if off == 0 {
                self.data.push(0);
            }
            let byte_idx = self.bit_idx / 8;
            let chunk_size = (8 - off).min(bits - written);
            let mask = (1u64 << chunk_size) - 1;
            let chunk = ((value >> written) & mask) as u8;
            self.data[byte_idx] |= chunk << off;
            self.bit_idx += chunk_size;
            written += chunk_size;
        }
    }

    /// Finalize the bit stream and return the packed bytes in reversed order.
    ///
    /// The reversal ensures the decoder can read the bit-packed region from
    /// the end of the encoded buffer. Consumes the helper.
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::{EncodeHelper, DecodeHelper};
    ///
    /// let mut enc = EncodeHelper::default();
    /// enc.write_partly(7u64, 3);
    /// enc.write_bit(true);
    ///
    /// let bytes = enc.finish();
    ///
    /// let mut dec = DecodeHelper::default();
    /// assert_eq!(dec.read_partly(&bytes, 3).unwrap(), 7);
    /// assert_eq!(dec.read_bit(&bytes).unwrap(), true);
    /// ```
    #[inline]
    pub fn finish(mut self) -> Vec<u8> {
        self.data.reverse();
        self.data
    }
}

/// Helper for decoding bit-packed fields during deserialization.
///
/// Used internally by the generated `Decode` implementations. You typically
/// do not need to use this directly unless you are implementing `Decode` manually.
///
/// Reads bits from the end of the encoded byte slice, matching the reversed
/// layout produced by [`EncodeHelper::finish`].
///
/// # Example
///
/// ```
/// use binsard::{EncodeHelper, DecodeHelper};
///
/// // Encode two values
/// let mut enc = EncodeHelper::default();
/// enc.write_partly(42u64, 6);
/// enc.write_bit(true);
/// let bytes = enc.finish();
///
/// // Decode them back
/// let mut dec = DecodeHelper::default();
/// assert_eq!(dec.read_partly(&bytes, 6).unwrap(), 42);
/// assert_eq!(dec.read_bit(&bytes).unwrap(), true);
/// ```
#[derive(Default)]
pub struct DecodeHelper {
    bit_idx: usize,
}

impl DecodeHelper {
    /// Skip `bits` positions in the bit stream.
    ///
    /// Used by the generated code to skip past reserved bits that were
    /// written by [`EncodeHelper::reserve_bits`].
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::{EncodeHelper, DecodeHelper};
    ///
    /// let mut enc = EncodeHelper::default();
    /// enc.reserve_bits(4);
    /// enc.write_partly(9u64, 4);
    /// let bytes = enc.finish();
    ///
    /// let mut dec = DecodeHelper::default();
    /// dec.reserve_bits(4); // skip the reserved region
    /// assert_eq!(dec.read_partly(&bytes, 4).unwrap(), 9);
    /// ```
    #[inline]
    pub const fn reserve_bits(&mut self, bits: usize) {
        self.bit_idx += bits;
    }

    /// Read a single bit from the bit stream and return it as a `bool`.
    ///
    /// Returns `true` if the bit is 1, `false` if 0. Advances the internal
    /// bit position by one.
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::{EncodeHelper, DecodeHelper};
    ///
    /// let mut enc = EncodeHelper::default();
    /// enc.write_bit(true);
    /// enc.write_bit(false);
    /// let bytes = enc.finish();
    ///
    /// let mut dec = DecodeHelper::default();
    /// assert_eq!(dec.read_bit(&bytes).unwrap(), true);
    /// assert_eq!(dec.read_bit(&bytes).unwrap(), false);
    /// ```
    #[inline]
    pub fn read_bit(&mut self, data: &[u8]) -> Result<bool, DecodeError> {
        let byte_offset = self.bit_idx / 8;
        if byte_offset >= data.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let off = self.bit_idx % 8;
        let byte_idx = data.len() - byte_offset - 1;
        let value = data[byte_idx] & (1 << off) != 0;
        self.bit_idx += 1;
        Ok(value)
    }

    /// Read `bits` bits from the bit stream and return them as a `u64`.
    ///
    /// Bits are read in LSB-first order, matching the layout written by
    /// [`EncodeHelper::write_partly`]. Cast the result to the target type
    /// (e.g., `as u8`, `as u16`).
    ///
    /// # Example
    ///
    /// ```
    /// use binsard::{EncodeHelper, DecodeHelper};
    ///
    /// let mut enc = EncodeHelper::default();
    /// enc.write_partly(3000u64, 12);
    /// let bytes = enc.finish();
    ///
    /// let mut dec = DecodeHelper::default();
    /// let value = dec.read_partly(&bytes, 12).unwrap() as u16;
    /// assert_eq!(value, 3000);
    /// ```
    #[inline]
    pub fn read_partly(&mut self, data: &[u8], bits: usize) -> Result<u64, DecodeError> {
        let total_bits_needed = self.bit_idx + bits;
        if total_bits_needed > data.len() * 8 {
            return Err(DecodeError::UnexpectedEof);
        }
        let mut value: u64 = 0;
        let mut bits_read = 0;
        while bits_read < bits {
            let off = self.bit_idx % 8;
            let byte_idx = data.len() - (self.bit_idx / 8) - 1;
            let chunk_size = (8 - off).min(bits - bits_read);
            let mask = ((1u16 << chunk_size) - 1) as u8;
            let chunk = (data[byte_idx] >> off) & mask;
            value |= (chunk as u64) << bits_read;
            self.bit_idx += chunk_size;
            bits_read += chunk_size;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_bits(enc_fn: impl FnOnce(&mut EncodeHelper), dec_fn: impl FnOnce(&mut DecodeHelper, &[u8])) {
        let mut enc = EncodeHelper::default();
        enc_fn(&mut enc);
        let data = enc.finish();
        let mut dec = DecodeHelper::default();
        dec_fn(&mut dec, &data);
    }

    #[test]
    fn write_bit_and_read_bit_roundtrip() {
        roundtrip_bits(
            |enc| {
                enc.write_bit(true);
                enc.write_bit(false);
                enc.write_bit(true);
            },
            |dec, data| {
                assert!(dec.read_bit(data).unwrap());
                assert!(!dec.read_bit(data).unwrap());
                assert!(dec.read_bit(data).unwrap());
            },
        );
    }

    #[test]
    fn write_partly_4bit_roundtrip() {
        roundtrip_bits(
            |enc| enc.write_partly(10u64, 4),
            |dec, data| assert_eq!(dec.read_partly(data, 4).unwrap() as u8, 10),
        );
    }

    #[test]
    fn write_partly_12bit_roundtrip() {
        roundtrip_bits(
            |enc| enc.write_partly(3000u64, 12),
            |dec, data| assert_eq!(dec.read_partly(data, 12).unwrap() as u16, 3000),
        );
    }

    #[test]
    fn write_partly_max_values() {
        roundtrip_bits(
            |enc| {
                enc.write_partly(0xFu64, 4);
                enc.write_partly(0xFFFu64, 12);
                enc.write_partly(0x1Fu64, 5);
            },
            |dec, data| {
                assert_eq!(dec.read_partly(data, 4).unwrap(), 0xF);
                assert_eq!(dec.read_partly(data, 12).unwrap(), 0xFFF);
                assert_eq!(dec.read_partly(data, 5).unwrap(), 0x1F);
            },
        );
    }

    #[test]
    fn write_partly_zero_values() {
        roundtrip_bits(
            |enc| {
                enc.write_partly(0u64, 4);
                enc.write_partly(0u64, 12);
            },
            |dec, data| {
                assert_eq!(dec.read_partly(data, 4).unwrap(), 0);
                assert_eq!(dec.read_partly(data, 12).unwrap(), 0);
            },
        );
    }

    #[test]
    fn write_partly_large_value_42bit() {
        let value: i64 = 1032123123;
        roundtrip_bits(
            |enc| enc.write_partly(value as u64, 42),
            |dec, data| assert_eq!(dec.read_partly(data, 42).unwrap() as i64, value),
        );
    }

    #[test]
    fn mixed_bits_and_partly() {
        roundtrip_bits(
            |enc| {
                enc.write_bit(true);
                enc.write_bit(false);
                enc.write_partly(10u64, 4);
                enc.write_partly(15u64, 5);
                enc.write_partly(1032123123u64, 42);
                enc.write_bit(true);
            },
            |dec, data| {
                assert!(dec.read_bit(data).unwrap());
                assert!(!dec.read_bit(data).unwrap());
                assert_eq!(dec.read_partly(data, 4).unwrap() as u8, 10);
                assert_eq!(dec.read_partly(data, 5).unwrap() as u8, 15);
                assert_eq!(dec.read_partly(data, 42).unwrap() as i64, 1032123123);
                assert!(dec.read_bit(data).unwrap());
            },
        );
    }

    #[test]
    fn bit_idx_tracks_correctly() {
        let mut enc = EncodeHelper::default();
        enc.write_bit(true);
        enc.write_bit(false);
        enc.write_partly(10u64, 4);
        enc.write_partly(15u64, 5);
        enc.write_partly(1032123123u64, 42);
        enc.write_bit(true);
        assert_eq!(enc.bit_idx, 54); // 1 + 1 + 4 + 5 + 42 + 1
    }

    #[test]
    fn reserve_bits_skips_on_both_sides() {
        let mut enc = EncodeHelper::default();
        enc.reserve_bits(3);
        enc.write_partly(7u64, 3);
        let data = enc.finish();

        let mut dec = DecodeHelper::default();
        dec.reserve_bits(3);
        assert_eq!(dec.read_partly(&data, 3).unwrap(), 7);
    }

    #[test]
    fn reserve_bits_zero_is_noop() {
        let mut enc = EncodeHelper::default();
        enc.reserve_bits(0);
        enc.write_bit(true);
        let data = enc.finish();

        let mut dec = DecodeHelper::default();
        dec.reserve_bits(0);
        assert!(dec.read_bit(&data).unwrap());
    }

    #[test]
    fn has_data_empty_vs_nonempty() {
        let mut enc = EncodeHelper::default();
        assert!(!enc.has_data());
        enc.write_bit(false);
        assert!(enc.has_data());
    }

    #[test]
    fn finish_returns_reversed_bytes() {
        let mut enc = EncodeHelper::default();
        enc.write_partly(0xABu64, 8);
        enc.write_partly(0xCDu64, 8);
        let data = enc.finish();
        assert_eq!(data, vec![0xCD, 0xAB]);
    }

    #[test]
    fn byte_boundary_crossing() {
        roundtrip_bits(
            |enc| {
                enc.write_partly(0b111u64, 3);
                enc.write_partly(0b11111111_11u64, 10);
            },
            |dec, data| {
                assert_eq!(dec.read_partly(data, 3).unwrap(), 0b111);
                assert_eq!(dec.read_partly(data, 10).unwrap(), 0b11111111_11);
            },
        );
    }

    #[test]
    fn single_bit_values_in_sequence() {
        roundtrip_bits(
            |enc| {
                for i in 0..16 {
                    enc.write_bit(i % 2 == 0);
                }
            },
            |dec, data| {
                for i in 0..16 {
                    assert_eq!(dec.read_bit(data).unwrap(), i % 2 == 0, "bit {i}");
                }
            },
        );
    }

    #[test]
    fn many_small_values_packed() {
        roundtrip_bits(
            |enc| {
                for i in 0..20u64 {
                    enc.write_partly(i % 4, 2);
                }
            },
            |dec, data| {
                for i in 0..20u64 {
                    assert_eq!(dec.read_partly(data, 2).unwrap(), i % 4, "value {i}");
                }
            },
        );
    }
}
