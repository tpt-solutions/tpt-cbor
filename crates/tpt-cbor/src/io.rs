//! `std::io` adapters (feature `std`).

use std::io::{self, Read};
use std::vec::Vec;

use crate::encode::{Encode, Encoder, IoSink};
use crate::error::EncodeError;

/// Encodes `value` canonically into `writer`.
///
/// Bytes are written as they are produced. On error, a partial encoding may
/// already have reached `writer`.
pub fn to_writer<T: Encode + ?Sized, W: io::Write>(
    value: &T,
    writer: W,
) -> Result<(), EncodeError> {
    let mut sink = IoSink(writer);
    let mut encoder = Encoder::new(&mut sink);
    value.encode(&mut encoder)
}

/// Reads `reader` to its end and returns the bytes.
///
/// Decoding is zero-copy only from a buffer, so streaming input is read into
/// memory first. Decode from the returned `Vec` with [`crate::from_slice`].
pub fn read_to_buffer<R: Read>(mut reader: R) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}
