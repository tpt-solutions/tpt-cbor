//! `std::io` adapters (feature `std`).

use std::fmt;
use std::io::{self, Read};
use std::vec::Vec;

use crate::decode::Decode;
use crate::encode::{Encode, Encoder, IoSink};
use crate::error::{DecodeError, EncodeError};
use crate::from_slice;

/// Why [`from_reader`] failed: the reader failed, or the bytes did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadError {
    /// The reader returned an I/O error. Only its kind is kept.
    Io(io::ErrorKind),
    /// The bytes were read but did not decode.
    Decode(DecodeError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(kind) => write!(f, "I/O error while reading: {kind:?}"),
            Self::Decode(error) => write!(f, "decode error: {error}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads one data item from `reader` in strict mode and returns an owned
/// value.
///
/// The reader is read to its end first, so it must hold exactly one item. The
/// type must not borrow from the input. Use [`from_slice`] with a borrowed
/// type for zero-copy decoding.
pub fn from_reader<T, R>(reader: R) -> Result<T, ReadError>
where
    T: for<'de> Decode<'de>,
    R: Read,
{
    let bytes = read_to_buffer(reader).map_err(|e| ReadError::Io(e.kind()))?;
    from_slice::<T>(&bytes).map_err(ReadError::Decode)
}

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
