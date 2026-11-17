//! Error types for encoding and decoding.

use core::fmt;

/// Errors produced while decoding CBOR.
///
/// Variants are unit-like and `Copy`, so they are cheap to pass around and do
/// not allocate. The enum is `#[non_exhaustive]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    /// Input ended before the item was complete, or a declared length is
    /// larger than the remaining input.
    UnexpectedEof,
    /// Container nesting exceeded the configured maximum depth.
    DepthExceeded,
    /// A value does not fit the requested type, or a length does not fit in
    /// `usize`.
    Overflow,
    /// A text string is not valid UTF-8.
    InvalidUtf8,
    /// An indefinite-length item was found. These are always rejected.
    IndefiniteLength,
    /// A valid encoding that is not the canonical one (strict mode only).
    NonCanonical,
    /// Map keys are not in canonical order (strict mode only).
    UnsortedMap,
    /// A map key appears more than once (strict mode only).
    DuplicateKey,
    /// A well-formed item outside the supported subset: tags, `undefined`,
    /// or unassigned simple values.
    UnsupportedType,
    /// The item's major type differs from what the caller asked for.
    TypeMismatch,
    /// Reserved additional-information value, or a stray break code.
    Malformed,
    /// Bytes remain after the top-level item.
    TrailingBytes,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::UnexpectedEof => "unexpected end of input",
            Self::DepthExceeded => "maximum nesting depth exceeded",
            Self::Overflow => "value out of range for target type",
            Self::InvalidUtf8 => "text string is not valid UTF-8",
            Self::IndefiniteLength => "indefinite-length items are not supported",
            Self::NonCanonical => "non-canonical encoding",
            Self::UnsortedMap => "map keys are not in canonical order",
            Self::DuplicateKey => "duplicate map key",
            Self::UnsupportedType => "unsupported item (tag, undefined or unassigned simple value)",
            Self::TypeMismatch => "CBOR major type does not match the requested type",
            Self::Malformed => "malformed item",
            Self::TrailingBytes => "trailing bytes after top-level item",
        };
        f.write_str(msg)
    }
}

impl core::error::Error for DecodeError {}

/// Errors produced while encoding CBOR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncodeError {
    /// Container nesting exceeded the maximum depth.
    DepthExceeded,
    /// A `Value::Map` contains two keys with identical encodings.
    DuplicateKey,
    /// The underlying `std::io::Write` failed. Only the error kind is kept so
    /// that the type stays `Copy` and `Eq`.
    #[cfg(feature = "std")]
    Io(std::io::ErrorKind),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DepthExceeded => f.write_str("maximum nesting depth exceeded"),
            Self::DuplicateKey => f.write_str("duplicate map key"),
            #[cfg(feature = "std")]
            Self::Io(kind) => write!(f, "I/O error while writing: {kind:?}"),
        }
    }
}

impl core::error::Error for EncodeError {}
