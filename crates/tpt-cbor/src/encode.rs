//! Canonical CBOR encoder and the `Encode` trait.

use crate::error::EncodeError;
use crate::float;
use crate::DEFAULT_MAX_DEPTH;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

// Initial-byte bases: major type already shifted into bits 5..=7.
pub(crate) const UINT: u8 = 0x00;
pub(crate) const NINT: u8 = 0x20;
pub(crate) const BYTES: u8 = 0x40;
pub(crate) const TEXT: u8 = 0x60;
pub(crate) const ARRAY: u8 = 0x80;
pub(crate) const MAP: u8 = 0xa0;

/// Destination for encoded bytes.
///
/// Implemented for `Vec<u8>` (with `alloc`) and, with `std`, for any
/// `std::io::Write` via `IoSink` (feature `std`).
pub trait Sink {
    /// Appends all of `bytes` to the sink.
    fn write_all_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError>;
}

#[cfg(feature = "alloc")]
impl Sink for Vec<u8> {
    fn write_all_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

/// Adapts a `std::io::Write` into a [`Sink`].
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct IoSink<W>(pub W);

#[cfg(feature = "std")]
impl<W: std::io::Write> Sink for IoSink<W> {
    fn write_all_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.0
            .write_all(bytes)
            .map_err(|e| EncodeError::Io(e.kind()))
    }
}

/// Streams canonical CBOR into a [`Sink`].
///
/// The encoder tracks container nesting so that output it produces can always
/// be decoded with the default decoder configuration.
pub struct Encoder<'a> {
    sink: &'a mut dyn Sink,
    depth: usize,
    max_depth: usize,
}

impl core::fmt::Debug for Encoder<'_> {
    /// Shows nesting state only. The sink is a trait object and is omitted.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Encoder")
            .field("depth", &self.depth)
            .field("max_depth", &self.max_depth)
            .finish_non_exhaustive()
    }
}

impl<'a> Encoder<'a> {
    /// Creates an encoder with the default maximum depth.
    pub fn new(sink: &'a mut dyn Sink) -> Self {
        Self {
            sink,
            depth: 0,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// Creates an encoder with a custom maximum depth (clamped to
    /// [`crate::MAX_DEPTH_LIMIT`]).
    pub fn with_max_depth(sink: &'a mut dyn Sink, max_depth: usize) -> Self {
        Self {
            sink,
            depth: 0,
            max_depth: max_depth.min(crate::MAX_DEPTH_LIMIT),
        }
    }

    /// Writes raw bytes to the sink. Callers are responsible for producing
    /// valid, canonical CBOR.
    pub(crate) fn write_raw(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.sink.write_all_bytes(bytes)
    }

    /// Writes an initial byte plus the minimal argument for `arg`.
    fn write_head(&mut self, base: u8, arg: u64) -> Result<(), EncodeError> {
        if arg < 24 {
            return self.write_raw(&[base | arg as u8]);
        }
        if let Ok(a) = u8::try_from(arg) {
            self.write_raw(&[base | 24])?;
            return self.write_raw(&[a]);
        }
        if let Ok(a) = u16::try_from(arg) {
            self.write_raw(&[base | 25])?;
            return self.write_raw(&a.to_be_bytes());
        }
        if let Ok(a) = u32::try_from(arg) {
            self.write_raw(&[base | 26])?;
            return self.write_raw(&a.to_be_bytes());
        }
        self.write_raw(&[base | 27])?;
        self.write_raw(&arg.to_be_bytes())
    }

    /// Encodes an unsigned integer in its minimal form.
    pub fn encode_u64(&mut self, value: u64) -> Result<(), EncodeError> {
        self.write_head(UINT, value)
    }

    /// Encodes a signed integer in its minimal form.
    pub fn encode_i64(&mut self, value: i64) -> Result<(), EncodeError> {
        if value >= 0 {
            self.write_head(UINT, value as u64)
        } else {
            // CBOR stores negative n as -1 - n. For negative value, `!value` is -1 - value.
            self.write_head(NINT, (!value) as u64)
        }
    }

    /// Encodes a byte string.
    pub fn encode_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.write_head(BYTES, bytes.len() as u64)?;
        self.write_raw(bytes)
    }

    /// Encodes a UTF-8 text string.
    pub fn encode_str(&mut self, text: &str) -> Result<(), EncodeError> {
        self.write_head(TEXT, text.len() as u64)?;
        self.write_raw(text.as_bytes())
    }

    /// Encodes a boolean (`0xf4` or `0xf5`).
    pub fn encode_bool(&mut self, value: bool) -> Result<(), EncodeError> {
        self.write_raw(&[if value { 0xf5 } else { 0xf4 }])
    }

    /// Encodes `null` (`0xf6`).
    pub fn encode_null(&mut self) -> Result<(), EncodeError> {
        self.write_raw(&[0xf6])
    }

    /// Encodes a float at its smallest lossless width. NaN becomes `0xf97e00`.
    pub fn encode_f64(&mut self, value: f64) -> Result<(), EncodeError> {
        float::encode(self, value)
    }

    /// Writes the header of a definite-length array of `len` items.
    ///
    /// The caller must then encode exactly `len` items.
    pub fn encode_array_header(&mut self, len: usize) -> Result<(), EncodeError> {
        self.write_head(ARRAY, len as u64)
    }

    /// Writes the header of a definite-length map of `len` pairs.
    ///
    /// The caller must then write `len` key/value pairs in canonical key order.
    /// [`crate::Value`] does this automatically; hand-written encoders must sort
    /// their own keys (RFC 8949 §4.2.1 bytewise order of the encoded keys).
    pub fn encode_map_header(&mut self, len: usize) -> Result<(), EncodeError> {
        self.write_head(MAP, len as u64)
    }

    /// Marks entry into a container. Must be paired with [`Self::leave_container`].
    pub fn enter_container(&mut self) -> Result<(), EncodeError> {
        let next = self
            .depth
            .checked_add(1)
            .ok_or(EncodeError::DepthExceeded)?;
        if next > self.max_depth {
            return Err(EncodeError::DepthExceeded);
        }
        self.depth = next;
        Ok(())
    }

    /// Marks exit from a container entered with [`Self::enter_container`].
    pub fn leave_container(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Creates an encoder writing into `sink` that inherits this encoder's
    /// depth state. Used to encode map keys into a scratch buffer for sorting.
    #[cfg(feature = "alloc")]
    pub(crate) fn nested<'b>(&self, sink: &'b mut dyn Sink) -> Encoder<'b> {
        Encoder {
            sink,
            depth: self.depth,
            max_depth: self.max_depth,
        }
    }
}

/// Types that can be written as canonical CBOR.
pub trait Encode {
    /// Writes `self` to `encoder`.
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError>;
}

impl Encode for bool {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_bool(*self)
    }
}

impl Encode for usize {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_u64(*self as u64)
    }
}

impl Encode for isize {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_i64(*self as i64)
    }
}

impl Encode for char {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        let mut buf = [0u8; 4];
        encoder.encode_str(self.encode_utf8(&mut buf))
    }
}

impl Encode for () {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_null()
    }
}

/// Encodes a map from `pairs`, sorting the entries by their encoded keys
/// (RFC 8949 §4.2.1). Two distinct Rust keys that encode to the same bytes
/// are a [`EncodeError::DuplicateKey`].
#[cfg(feature = "alloc")]
pub(crate) fn encode_pairs<'p, K, V, I>(
    pairs: I,
    len: usize,
    encoder: &mut Encoder<'_>,
) -> Result<(), EncodeError>
where
    K: Encode + 'p,
    V: Encode + 'p,
    I: Iterator<Item = (&'p K, &'p V)>,
{
    encoder.enter_container()?;
    let mut keyed: alloc::vec::Vec<(alloc::vec::Vec<u8>, &V)> = alloc::vec::Vec::with_capacity(len);
    for (key, value) in pairs {
        let mut key_bytes = alloc::vec::Vec::new();
        {
            let mut key_encoder = encoder.nested(&mut key_bytes);
            key.encode(&mut key_encoder)?;
        }
        keyed.push((key_bytes, value));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    for window in keyed.windows(2) {
        if let [a, b] = window {
            if a.0 == b.0 {
                return Err(EncodeError::DuplicateKey);
            }
        }
    }
    encoder.encode_map_header(keyed.len())?;
    for (key_bytes, value) in &keyed {
        encoder.write_raw(key_bytes)?;
        value.encode(encoder)?;
    }
    encoder.leave_container();
    Ok(())
}

macro_rules! impl_encode_unsigned {
    ($($t:ty),*) => {$(
        impl Encode for $t {
            fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
                encoder.encode_u64(*self as u64)
            }
        }
    )*};
}
impl_encode_unsigned!(u8, u16, u32, u64);

macro_rules! impl_encode_signed {
    ($($t:ty),*) => {$(
        impl Encode for $t {
            fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
                encoder.encode_i64(*self as i64)
            }
        }
    )*};
}
impl_encode_signed!(i8, i16, i32, i64);

impl Encode for f64 {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_f64(*self)
    }
}

impl Encode for f32 {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        // f32 -> f64 is exact, so the canonical width is chosen on the true value.
        encoder.encode_f64(f64::from(*self))
    }
}

impl Encode for str {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_str(self)
    }
}

impl Encode for &str {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_str(self)
    }
}

/// Byte strings are encoded as CBOR byte strings (major type 2).
///
/// `Vec<u8>` is encoded as an array of integers, like every `Vec<T>`. Use a
/// `&[u8]` to produce a byte string.
impl Encode for &[u8] {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_bytes(self)
    }
}

impl Encode for [u8] {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_bytes(self)
    }
}

#[cfg(feature = "alloc")]
impl<T: Encode + ?Sized> Encode for alloc::boxed::Box<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        T::encode(self, encoder)
    }
}

#[cfg(feature = "alloc")]
impl<T: alloc::borrow::ToOwned + Encode + ?Sized> Encode for alloc::borrow::Cow<'_, T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        T::encode(self, encoder)
    }
}

#[cfg(feature = "alloc")]
impl<K: Encode, V: Encode> Encode for alloc::collections::BTreeMap<K, V> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encode_pairs(self.iter(), self.len(), encoder)
    }
}

#[cfg(feature = "std")]
impl<K: Encode, V: Encode, S> Encode for std::collections::HashMap<K, V, S> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encode_pairs(self.iter(), self.len(), encoder)
    }
}

#[cfg(feature = "alloc")]
impl Encode for String {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_str(self.as_str())
    }
}

#[cfg(feature = "alloc")]
impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.enter_container()?;
        encoder.encode_array_header(self.len())?;
        for item in self {
            item.encode(encoder)?;
        }
        encoder.leave_container();
        Ok(())
    }
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.enter_container()?;
        encoder.encode_array_header(N)?;
        for item in self {
            item.encode(encoder)?;
        }
        encoder.leave_container();
        Ok(())
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        match self {
            Some(value) => value.encode(encoder),
            None => encoder.encode_null(),
        }
    }
}

macro_rules! impl_encode_tuple {
    ($len:expr; $($idx:tt : $name:ident),+) => {
        impl<$($name: Encode),+> Encode for ($($name,)+) {
            fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
                encoder.enter_container()?;
                encoder.encode_array_header($len)?;
                $( self.$idx.encode(encoder)?; )+
                encoder.leave_container();
                Ok(())
            }
        }
    };
}
impl_encode_tuple!(2; 0: A, 1: B);
impl_encode_tuple!(3; 0: A, 1: B, 2: C);
impl_encode_tuple!(4; 0: A, 1: B, 2: C, 3: D);
impl_encode_tuple!(5; 0: A, 1: B, 2: C, 3: D, 4: E);
impl_encode_tuple!(6; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_encode_tuple!(7; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_encode_tuple!(8; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
