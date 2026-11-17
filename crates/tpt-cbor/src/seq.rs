//! RFC 8742 CBOR sequences: data items laid end to end with no framing.

use core::marker::PhantomData;

use crate::decode::{Decode, Decoder};
use crate::error::DecodeError;
use crate::Config;

/// Iterator over the items of a CBOR sequence. Created by [`sequence`] or
/// [`sequence_with_config`].
///
/// Each call to `next` decodes one item. Iteration ends after the last item,
/// or after the first error (the error is yielded once).
pub struct Sequence<'a, T> {
    decoder: Decoder<'a>,
    done: bool,
    marker: PhantomData<T>,
}

impl<'a, T: Decode<'a>> Iterator for Sequence<'a, T> {
    type Item = Result<T, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.decoder.remaining() == 0 {
            return None;
        }
        let item = T::decode(&mut self.decoder);
        if item.is_err() {
            self.done = true;
        }
        Some(item)
    }
}

/// Iterates over the items of a CBOR sequence in strict mode.
pub fn sequence<'a, T: Decode<'a>>(input: &'a [u8]) -> Sequence<'a, T> {
    sequence_with_config(input, Config::strict())
}

/// Iterates over the items of a CBOR sequence with an explicit [`Config`].
pub fn sequence_with_config<'a, T: Decode<'a>>(input: &'a [u8], config: Config) -> Sequence<'a, T> {
    Sequence {
        decoder: Decoder::with_config(input, config),
        done: false,
        marker: PhantomData,
    }
}
