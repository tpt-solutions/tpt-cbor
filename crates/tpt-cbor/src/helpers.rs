//! Whole-input helpers: validation, canonicality checks and canonicalization.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::decode::Decoder;
use crate::error::Located;
#[cfg(feature = "alloc")]
use crate::error::{DecodeError, EncodeError};
#[cfg(not(feature = "alloc"))]
use crate::Config;
#[cfg(feature = "alloc")]
use crate::{from_slice_with_config, to_vec, Config, Value};

/// Checks that `input` is exactly one well-formed data item under `config`.
///
/// Nothing is allocated. Trailing bytes are an error. Use [`Config::strict`]
/// to reject non-canonical input, or [`Config::lenient`] to accept it.
pub fn validate(input: &[u8], config: Config) -> Result<(), Located> {
    let mut decoder = Decoder::with_config(input, config);
    let result = decoder.skip_item().and_then(|()| decoder.finish());
    result.map_err(|error| Located {
        error,
        offset: decoder.position(),
    })
}

/// `true` if `input` is exactly one canonical data item (strict validation).
pub fn is_canonical(input: &[u8]) -> bool {
    validate(input, Config::strict()).is_ok()
}

/// Re-encodes `input` in canonical form.
///
/// The input is decoded leniently and re-encoded, so it may be non-canonical.
/// It is decoded into a [`Value`], which has the same limits: integers must fit
/// `i64`, and tags, `undefined` and unassigned simple values are rejected.
#[cfg(feature = "alloc")]
pub fn canonicalize(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let value: Value<'_> = from_slice_with_config(input, Config::lenient())?;
    // `EncodeError` is non_exhaustive for downstream code. Inside this crate
    // the catch-all may look unreachable, so the lint is allowed here.
    #[allow(unreachable_patterns)]
    to_vec(&value).map_err(|error| match error {
        EncodeError::DepthExceeded => DecodeError::DepthExceeded,
        EncodeError::DuplicateKey => DecodeError::DuplicateKey,
        _ => DecodeError::Malformed,
    })
}
