//! Owned byte strings.

use alloc::vec::Vec;
use core::ops::Deref;

use crate::decode::{Decode, Decoder};
use crate::encode::{Encode, Encoder};
use crate::error::{DecodeError, EncodeError};

/// An owned CBOR byte string.
///
/// `Vec<u8>` encodes as an array of integers, like every `Vec<T>`. Use
/// `ByteBuf` for a byte string that owns its data, or `&[u8]` for a borrowed
/// one. Derived structs can also use `#[cbor(bytes)]` on a `Vec<u8>` field.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteBuf(pub Vec<u8>);

impl ByteBuf {
    /// Returns the underlying bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for ByteBuf {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl Deref for ByteBuf {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for ByteBuf {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Encode for ByteBuf {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_bytes(&self.0)
    }
}

impl<'a> Decode<'a> for ByteBuf {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_bytes().map(|bytes| Self(bytes.to_vec()))
    }
}
