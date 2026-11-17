//! # tpt-cbor
//!
//! A pure-Rust, zero-dependency implementation of CBOR (RFC 8949).
//!
//! * **Strict canonical encoding.** Integers use the shortest form, floats use
//!   the shortest lossless width (NaN is always `0xf97e00`), lengths are
//!   definite, and map keys are sorted by RFC 8949 §4.2.1 bytewise order.
//! * **Strict decoding by default.** Non-canonical input is rejected unless a
//!   [`Config::lenient`] decoder is used. Indefinite-length items are always
//!   rejected, as are tags and unassigned simple values.
//! * **Zero-copy decoding.** Strings and byte strings borrow from the input.
//! * **Panic-free and bounded.** Nesting depth is capped (default 64) and all
//!   arithmetic is checked. The crate is `#![forbid(unsafe_code)]`.
//!
//! ## Example
//!
//! ```
//! use tpt_cbor::from_slice;
//!
//! // 1000 is 0x19 0x03 0xe8 in canonical form.
//! assert_eq!(from_slice::<u64>(&[0x19, 0x03, 0xe8]), Ok(1000));
//!
//! // Non-minimal encodings are rejected in strict mode (the default).
//! assert!(from_slice::<u64>(&[0x18, 0x01]).is_err());
//! ```
//!
//! ## Feature flags
//!
//! | Feature | Default | Effect |
//! |---------|---------|--------|
//! | `alloc` | yes     | `Value`, `Vec`/`String` codecs, `to_vec` |
//! | `std`   | no      | `std::io` adapters (`to_writer`, `read_to_buffer`); implies `alloc` |
//!
//! Without any features the crate is `#![no_std]` and needs no allocator.
#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(
    not(test),
    deny(
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::arithmetic_side_effects
    )
)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod decode;
mod encode;
mod error;
mod float;

#[cfg(feature = "alloc")]
mod value;

#[cfg(feature = "std")]
mod io;

#[cfg(all(kani, feature = "alloc"))]
mod proofs;

pub use decode::{Config, Decode, Decoder, Kind};
pub use encode::{Encode, Encoder, Sink};
pub use error::{DecodeError, EncodeError};

#[cfg(feature = "alloc")]
pub use value::Value;

#[cfg(feature = "std")]
pub use encode::IoSink;

#[cfg(feature = "std")]
pub use io::{read_to_buffer, to_writer};

#[cfg(feature = "derive")]
pub use tpt_cbor_derive::{Decode, Encode};

/// Default maximum container nesting depth accepted by the decoder and
/// produced by the encoder.
pub const DEFAULT_MAX_DEPTH: usize = 64;

/// Hard ceiling on nesting depth. Configured depths above this are clamped,
/// which keeps recursion bounded regardless of user configuration.
pub const MAX_DEPTH_LIMIT: usize = 256;

/// Encodes `value` into a new canonical byte vector.
#[cfg(feature = "alloc")]
pub fn to_vec<T: Encode + ?Sized>(value: &T) -> Result<alloc::vec::Vec<u8>, EncodeError> {
    let mut out = alloc::vec::Vec::new();
    {
        let mut encoder = Encoder::new(&mut out);
        value.encode(&mut encoder)?;
    }
    Ok(out)
}

/// Decodes a single top-level item from `input` using strict mode.
///
/// Returns [`DecodeError::TrailingBytes`] if any bytes follow the item.
pub fn from_slice<'a, T: Decode<'a>>(input: &'a [u8]) -> Result<T, DecodeError> {
    from_slice_with_config(input, Config::default())
}

/// Like [`from_slice`], but with an explicit [`Config`].
pub fn from_slice_with_config<'a, T: Decode<'a>>(
    input: &'a [u8],
    config: Config,
) -> Result<T, DecodeError> {
    let mut decoder = Decoder::with_config(input, config);
    let value = T::decode(&mut decoder)?;
    decoder.finish()?;
    Ok(value)
}
