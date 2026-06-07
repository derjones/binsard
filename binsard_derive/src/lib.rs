//! Procedural derive macros for the [`binsard`](https://docs.rs/binsard) crate.
//!
//! Provides `#[derive(Encode)]` and `#[derive(Decode)]` for compact binary
//! serialization with bit-packing support. You should not depend on this crate
//! directly -- use `binsard` instead, which re-exports the macros.

use proc_macro::TokenStream;

mod parser;
mod encode;
mod decode;

/// Derive the `Encode` trait for a struct or enum.
///
/// Generates an implementation of `binsard::Encode` that serializes all fields
/// into a compact binary format. Booleans and `Option` discriminants are
/// bit-packed automatically. Use `#[binsard(bits = N)]` on integer fields
/// to pack them into exactly N bits.
///
/// # Attributes
///
/// **Type-level** (on structs or enums):
/// - `#[binsard(bits = N)]` -- set the enum discriminant tag width (default: 8)
///
/// **Field-level** (on integer fields):
/// - `#[binsard(bits = N)]` -- pack this field into N bits instead of its full byte width
///
/// **Field-level** (on `String`, `Vec<u8>`, `Vec<T>`):
/// - `#[binsard(len_bytes = N)]` -- override the length prefix width (1, 2, or 4 bytes)
///
/// # Examples
///
/// Basic struct:
///
/// ```ignore
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// let p = Point { x: 10, y: -5 };
/// assert_eq!(p.encode().len(), 8);
/// ```
///
/// Bit-packed fields:
///
/// ```ignore
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// struct Sensor {
///     #[binsard(bits = 4)]
///     channel: u8,
///     #[binsard(bits = 12)]
///     value: u16,
///     active: bool,
/// }
///
/// let s = Sensor { channel: 9, value: 3000, active: true };
/// assert_eq!(s.encode().len(), 3); // 17 bits packed into 3 bytes
/// ```
///
/// Enum with custom tag width:
///
/// ```ignore
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// #[binsard(bits = 2)]
/// enum Priority { Low, Medium, High }
///
/// assert_eq!(Priority::High.encode().len(), 1); // 2-bit tag in 1 byte
/// ```
#[proc_macro_derive(Encode, attributes(binsard))]
pub fn derive_encode(item: TokenStream) -> TokenStream {
    let ast = match syn::parse(item) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error().into(),
    };

    encode::impl_encode_trait(ast)
}

/// Derive the `Decode` trait for a struct or enum.
///
/// Generates an implementation of `binsard::Decode` that deserializes a value
/// from a byte slice produced by the corresponding `Encode` implementation.
///
/// # Attributes
///
/// Same as [`derive_encode`] -- both `Encode` and `Decode` must use identical
/// `#[binsard(...)]` attributes to ensure a matching binary layout.
///
/// # Examples
///
/// ```ignore
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// struct Pair(u32, u8);
///
/// let original = Pair(12345, 42);
/// let bytes = original.encode();
/// let decoded = Pair::decode(&bytes);
/// assert_eq!(original, decoded);
/// ```
///
/// Enum with mixed variants:
///
/// ```ignore
/// use binsard::{Encode, Decode};
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// struct Payload(Vec<u8>);
///
/// #[derive(Encode, Decode, Debug, PartialEq)]
/// enum Command {
///     Ping,
///     Move { x: i32, y: i32 },
///     Send(Payload),
/// }
///
/// let cmd = Command::Move { x: -10, y: 42 };
/// let bytes = cmd.encode();
/// assert_eq!(cmd, Command::decode(&bytes));
/// ```
#[proc_macro_derive(Decode, attributes(binsard))]
pub fn derive_decode(item: TokenStream) -> TokenStream {
    let ast = match syn::parse(item) {
        Ok(ast) => ast,
        Err(err) => return err.to_compile_error().into(),
    };

    decode::impl_decode_trait(ast)
}
