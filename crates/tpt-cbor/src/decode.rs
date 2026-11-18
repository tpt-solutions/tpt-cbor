//! Strict, zero-copy CBOR decoder and the `Decode` trait.

use core::cmp::Ordering;

use crate::error::DecodeError;
use crate::float::{self, Canon};
use crate::{DEFAULT_MAX_DEPTH, DEFAULT_MAX_ITEMS, MAX_DEPTH_LIMIT};

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

/// Validates a text string's bytes as UTF-8.
#[cfg(not(kani))]
fn utf8(bytes: &[u8]) -> Result<&str, DecodeError> {
    core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8)
}

/// Under Kani, UTF-8 validation is modelled as an arbitrary accept or reject.
/// The validation loop dominates proof time, and the panic and bounds proofs
/// do not depend on string contents. Real validation is covered by the tests
/// and the fuzzer.
#[cfg(kani)]
fn utf8(_bytes: &[u8]) -> Result<&str, DecodeError> {
    if kani::any() {
        Ok("")
    } else {
        Err(DecodeError::InvalidUtf8)
    }
}

/// Upper bound on the capacity reserved up front for a decoded container.
///
/// The declared length is checked against the remaining input, but one input
/// byte can describe a container element that occupies many bytes in memory.
/// Reserving at most this many slots keeps a hostile header from forcing a
/// large allocation before any element has been decoded.
#[cfg(feature = "alloc")]
pub(crate) const PREALLOC_LIMIT: usize = 1024;

// Major type numbers (initial byte >> 5).
const MAJOR_UINT: u8 = 0;
const MAJOR_NINT: u8 = 1;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
const MAJOR_ARRAY: u8 = 4;
const MAJOR_MAP: u8 = 5;
const MAJOR_TAG: u8 = 6;
const MAJOR_SIMPLE: u8 = 7;

// Additional-information values with special meaning.
const INFO_INDEFINITE: u8 = 31;

/// Decoder behaviour.
///
/// The default is strict. Indefinite-length items, tags and unassigned simple
/// values are rejected in both modes. Strict mode additionally rejects
/// non-canonical encodings. Lenient mode accepts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// Reject non-canonical integers, floats, unsorted map keys and duplicate
    /// keys when `true`.
    pub strict: bool,
    /// Maximum container nesting depth. Values above
    /// [`crate::MAX_DEPTH_LIMIT`] are clamped.
    pub max_depth: usize,
    /// Maximum number of elements (array) or pairs (map) in any single
    /// container. A declared length above this fails with
    /// [`DecodeError::LimitExceeded`] before any element is read.
    ///
    /// This limits each container, not the total item count across a
    /// document. Nesting is bounded separately by `max_depth`.
    pub max_items: usize,
}

impl Config {
    /// Strict mode with the default limits.
    pub const fn strict() -> Self {
        Self {
            strict: true,
            max_depth: DEFAULT_MAX_DEPTH,
            max_items: DEFAULT_MAX_ITEMS,
        }
    }

    /// Lenient mode with the default limits.
    pub const fn lenient() -> Self {
        Self {
            strict: false,
            max_depth: DEFAULT_MAX_DEPTH,
            max_items: DEFAULT_MAX_ITEMS,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::strict()
    }
}

/// The broad category of the next item, as reported by [`Decoder::peek_kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Unsigned integer (major type 0).
    Unsigned,
    /// Negative integer (major type 1).
    Negative,
    /// Byte string (major type 2).
    Bytes,
    /// Text string (major type 3).
    Text,
    /// Array (major type 4).
    Array,
    /// Map (major type 5).
    Map,
    /// `true` or `false`.
    Bool,
    /// `null`.
    Null,
    /// Half, single or double precision float.
    Float,
}

/// A parsed argument: either a value, or the indefinite-length marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arg {
    Value(u64),
    Indefinite,
}

/// Zero-copy decoder over a borrowed byte slice.
///
/// After an error, the decoder must be discarded rather than resumed.
/// [`Decoder::position`] still reports where decoding stopped, which is used
/// for error offsets (see [`crate::Located`]).
pub struct Decoder<'a> {
    input: &'a [u8],
    offset: usize,
    depth: usize,
    config: Config,
}

impl core::fmt::Debug for Decoder<'_> {
    /// Shows position, depth and configuration. The input bytes are omitted,
    /// so large or sensitive payloads do not end up in logs.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Decoder")
            .field("input_len", &self.input.len())
            .field("offset", &self.offset)
            .field("depth", &self.depth)
            .field("config", &self.config)
            .finish()
    }
}

impl<'a> Decoder<'a> {
    /// Creates a strict decoder over `input`.
    pub fn new(input: &'a [u8]) -> Self {
        Self::with_config(input, Config::default())
    }

    /// Creates a decoder with an explicit configuration.
    pub fn with_config(input: &'a [u8], config: Config) -> Self {
        let config = Config {
            max_depth: config.max_depth.min(MAX_DEPTH_LIMIT),
            ..config
        };
        Self {
            input,
            offset: 0,
            depth: 0,
            config,
        }
    }

    /// The active configuration.
    pub fn config(&self) -> Config {
        self.config
    }

    /// Byte offset of the next unread byte.
    ///
    /// After a failed decode this is where decoding stopped: the first byte
    /// that could not be accepted, or the end of a header or key whose check
    /// failed. It is not always the exact first byte of the faulty item.
    pub fn position(&self) -> usize {
        self.offset
    }

    /// Number of unread bytes.
    pub fn remaining(&self) -> usize {
        self.input.len().saturating_sub(self.offset)
    }

    /// Succeeds only if all input has been consumed.
    pub fn finish(&self) -> Result<(), DecodeError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes)
        }
    }

    /// Classifies the next item without consuming it.
    ///
    /// Returns an error for tags, `undefined`, unassigned simple values and
    /// malformed or indefinite-length heads.
    pub fn peek_kind(&self) -> Result<Kind, DecodeError> {
        let initial = self.peek_byte()?;
        let major = initial >> 5;
        let info = initial & 0x1f;
        match (major, info) {
            (_, 28..=30) => Err(DecodeError::Malformed),
            (MAJOR_BYTES..=MAJOR_MAP, INFO_INDEFINITE) => Err(DecodeError::IndefiniteLength),
            (MAJOR_UINT | MAJOR_NINT | MAJOR_TAG | MAJOR_SIMPLE, INFO_INDEFINITE) => {
                Err(DecodeError::Malformed)
            }
            (MAJOR_UINT, _) => Ok(Kind::Unsigned),
            (MAJOR_NINT, _) => Ok(Kind::Negative),
            (MAJOR_BYTES, _) => Ok(Kind::Bytes),
            (MAJOR_TEXT, _) => Ok(Kind::Text),
            (MAJOR_ARRAY, _) => Ok(Kind::Array),
            (MAJOR_MAP, _) => Ok(Kind::Map),
            (MAJOR_TAG, _) => Err(DecodeError::UnsupportedType),
            (MAJOR_SIMPLE, 20 | 21) => Ok(Kind::Bool),
            (MAJOR_SIMPLE, 22) => Ok(Kind::Null),
            (MAJOR_SIMPLE, 25..=27) => Ok(Kind::Float),
            // Simple values 0-19, 23 (undefined) and 24 (one-byte simple).
            _ => Err(DecodeError::UnsupportedType),
        }
    }

    /// Decodes an unsigned integer.
    pub fn decode_u64(&mut self) -> Result<u64, DecodeError> {
        self.read_value_of(MAJOR_UINT)
    }

    /// Decodes a signed integer. Fails with [`DecodeError::Overflow`] if the
    /// value is outside `i64`.
    pub fn decode_i64(&mut self) -> Result<i64, DecodeError> {
        let (major, arg) = self.read_head()?;
        match (major, arg) {
            (MAJOR_UINT, Arg::Value(v)) => i64::try_from(v).map_err(|_| DecodeError::Overflow),
            // Negative n encodes -1 - n. `!x` is -1 - x for i64.
            (MAJOR_NINT, Arg::Value(v)) => i64::try_from(v)
                .map(|x| !x)
                .map_err(|_| DecodeError::Overflow),
            _ => Err(DecodeError::TypeMismatch),
        }
    }

    /// Decodes a byte string, borrowing from the input.
    pub fn decode_bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let n = self.read_value_of(MAJOR_BYTES)?;
        let len = usize::try_from(n).map_err(|_| DecodeError::UnexpectedEof)?;
        self.take(len)
    }

    /// Decodes a UTF-8 text string, borrowing from the input.
    pub fn decode_str(&mut self) -> Result<&'a str, DecodeError> {
        let n = self.read_value_of(MAJOR_TEXT)?;
        let len = usize::try_from(n).map_err(|_| DecodeError::UnexpectedEof)?;
        let bytes = self.take(len)?;
        utf8(bytes)
    }

    /// Decodes a boolean.
    pub fn decode_bool(&mut self) -> Result<bool, DecodeError> {
        match self.peek_kind()? {
            Kind::Bool => Ok(self.read_byte()? == 0xf5),
            _ => Err(DecodeError::TypeMismatch),
        }
    }

    /// Decodes `null`.
    pub fn decode_null(&mut self) -> Result<(), DecodeError> {
        match self.peek_kind()? {
            Kind::Null => {
                self.read_byte()?;
                Ok(())
            }
            _ => Err(DecodeError::TypeMismatch),
        }
    }

    /// Decodes a float of any width. Strict mode also checks that the width is
    /// the canonical one.
    pub fn decode_f64(&mut self) -> Result<f64, DecodeError> {
        let initial = self.read_byte()?;
        if initial >> 5 != MAJOR_SIMPLE {
            return Err(DecodeError::TypeMismatch);
        }
        let (value, raw) = match initial & 0x1f {
            25 => {
                let bits = self.read_be(2)? as u16;
                (float::half_to_f64(bits), Canon::Half(bits))
            }
            26 => {
                let bits = self.read_be(4)? as u32;
                (f64::from(f32::from_bits(bits)), Canon::Single(bits))
            }
            27 => {
                let bits = self.read_be(8)?;
                (f64::from_bits(bits), Canon::Double(bits))
            }
            28..=31 => return Err(DecodeError::Malformed),
            _ => return Err(DecodeError::TypeMismatch),
        };
        if self.config.strict && float::canonical(value) != raw {
            return Err(DecodeError::NonCanonical);
        }
        Ok(value)
    }

    /// Reads an array header and returns its element count.
    ///
    /// Each element occupies at least one byte, so a count larger than the
    /// remaining input is rejected before any allocation happens.
    pub fn decode_array_len(&mut self) -> Result<usize, DecodeError> {
        let n = self.read_value_of(MAJOR_ARRAY)?;
        let len = usize::try_from(n).map_err(|_| DecodeError::UnexpectedEof)?;
        if len > self.remaining() {
            return Err(DecodeError::UnexpectedEof);
        }
        if len > self.config.max_items {
            return Err(DecodeError::LimitExceeded);
        }
        Ok(len)
    }

    /// Reads a map header and returns its pair count.
    ///
    /// Each pair occupies at least two bytes, so the count is sanity-checked
    /// against the remaining input.
    pub fn decode_map_len(&mut self) -> Result<usize, DecodeError> {
        let n = self.read_value_of(MAJOR_MAP)?;
        let len = usize::try_from(n).map_err(|_| DecodeError::UnexpectedEof)?;
        let needed = len.checked_mul(2).ok_or(DecodeError::UnexpectedEof)?;
        if needed > self.remaining() {
            return Err(DecodeError::UnexpectedEof);
        }
        if len > self.config.max_items {
            return Err(DecodeError::LimitExceeded);
        }
        Ok(len)
    }

    /// Consumes one complete item without allocating.
    ///
    /// Applies the same validation as a full decode: canonical form in strict
    /// mode, UTF-8, depth limits, and key order and duplicates in maps.
    pub fn skip_item(&mut self) -> Result<(), DecodeError> {
        match self.peek_kind()? {
            Kind::Unsigned | Kind::Negative => {
                self.read_head()?;
            }
            Kind::Bytes => {
                self.decode_bytes()?;
            }
            Kind::Text => {
                self.decode_str()?;
            }
            Kind::Bool => {
                self.decode_bool()?;
            }
            Kind::Null => self.decode_null()?,
            Kind::Float => {
                self.decode_f64()?;
            }
            Kind::Array => {
                let n = self.decode_array_len()?;
                self.enter_container()?;
                for _ in 0..n {
                    self.skip_item()?;
                }
                self.leave_container();
            }
            Kind::Map => {
                let n = self.decode_map_len()?;
                self.enter_container()?;
                let mut prev: Option<(usize, usize)> = None;
                for _ in 0..n {
                    let key_start = self.offset;
                    self.skip_item()?;
                    let key = (key_start, self.offset);
                    self.check_key_order(prev, key)?;
                    prev = Some(key);
                    self.skip_item()?;
                }
                self.leave_container();
            }
        }
        Ok(())
    }

    /// Marks entry into a container. Fails with [`DecodeError::DepthExceeded`]
    /// past the configured limit. Pair with [`Self::leave_container`].
    pub fn enter_container(&mut self) -> Result<(), DecodeError> {
        let next = self
            .depth
            .checked_add(1)
            .ok_or(DecodeError::DepthExceeded)?;
        if next > self.config.max_depth {
            return Err(DecodeError::DepthExceeded);
        }
        self.depth = next;
        Ok(())
    }

    /// Marks exit from a container.
    pub fn leave_container(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Checks that a map key follows the previous key in canonical order.
    ///
    /// `prev` and `cur` are byte spans of the two encoded keys in the input,
    /// obtained from [`Decoder::position`]. Used by hand-written and derived
    /// map decoders. Only enforced in strict mode.
    ///
    /// A span outside the input is a caller error, not truncated data, so it
    /// is reported as [`DecodeError::Malformed`].
    pub fn check_key_order(
        &self,
        prev: Option<(usize, usize)>,
        cur: (usize, usize),
    ) -> Result<(), DecodeError> {
        if !self.config.strict {
            return Ok(());
        }
        let Some((ps, pe)) = prev else {
            return Ok(());
        };
        let (cs, ce) = cur;
        let a = self.input.get(ps..pe).ok_or(DecodeError::Malformed)?;
        let b = self.input.get(cs..ce).ok_or(DecodeError::Malformed)?;
        // Slice Ord is lexicographic on bytes, with a shorter prefix sorting first.
        // That is exactly RFC 8949 §4.2.1 bytewise order.
        match a.cmp(b) {
            Ordering::Less => Ok(()),
            Ordering::Equal => Err(DecodeError::DuplicateKey),
            Ordering::Greater => Err(DecodeError::UnsortedMap),
        }
    }

    // ----- internals -------------------------------------------------------

    fn peek_byte(&self) -> Result<u8, DecodeError> {
        self.input
            .get(self.offset)
            .copied()
            .ok_or(DecodeError::UnexpectedEof)
    }

    fn read_byte(&mut self) -> Result<u8, DecodeError> {
        let byte = self.peek_byte()?;
        self.offset = self
            .offset
            .checked_add(1)
            .ok_or(DecodeError::UnexpectedEof)?;
        Ok(byte)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .offset
            .checked_add(n)
            .ok_or(DecodeError::UnexpectedEof)?;
        let bytes = self
            .input
            .get(self.offset..end)
            .ok_or(DecodeError::UnexpectedEof)?;
        self.offset = end;
        Ok(bytes)
    }

    /// Reads `n` bytes as a big-endian unsigned integer (n <= 8).
    ///
    /// Only the widths 1, 2, 4 and 8 occur. Each is a fixed-size pattern, so
    /// there is no loop whose bound depends on the input. That keeps model
    /// checking cheap, and it is also the fast path for the decoder.
    fn read_be(&mut self, n: usize) -> Result<u64, DecodeError> {
        let bytes = self.take(n)?;
        match bytes {
            [b] => Ok(u64::from(*b)),
            [a, b] => Ok(u64::from(u16::from_be_bytes([*a, *b]))),
            [a, b, c, d] => Ok(u64::from(u32::from_be_bytes([*a, *b, *c, *d]))),
            [a, b, c, d, e, f, g, h] => Ok(u64::from_be_bytes([*a, *b, *c, *d, *e, *f, *g, *h])),
            _ => Err(DecodeError::Malformed),
        }
    }

    /// Reads an initial byte and its argument. Indefinite-length markers are
    /// returned as [`Arg::Indefinite`] and rejected by the caller.
    fn read_head(&mut self) -> Result<(u8, Arg), DecodeError> {
        let initial = self.read_byte()?;
        let major = initial >> 5;
        let arg = self.read_arg(major, initial & 0x1f)?;
        // Indefinite length is only meaningful for strings, arrays and maps.
        if arg == Arg::Indefinite && !(MAJOR_BYTES..=MAJOR_MAP).contains(&major) {
            return Err(DecodeError::Malformed);
        }
        Ok((major, arg))
    }

    fn read_arg(&mut self, major: u8, info: u8) -> Result<Arg, DecodeError> {
        let value = match info {
            0..=23 => u64::from(info),
            24 => {
                let v = self.read_be(1)?;
                self.check_minimal(major, info, v)?;
                v
            }
            25 => {
                let v = self.read_be(2)?;
                self.check_minimal(major, info, v)?;
                v
            }
            26 => {
                let v = self.read_be(4)?;
                self.check_minimal(major, info, v)?;
                v
            }
            27 => {
                let v = self.read_be(8)?;
                self.check_minimal(major, info, v)?;
                v
            }
            INFO_INDEFINITE => return Ok(Arg::Indefinite),
            // 28..=30 are reserved.
            _ => return Err(DecodeError::Malformed),
        };
        Ok(Arg::Value(value))
    }

    /// Strict mode: the argument must use the shortest width. Simple values
    /// (major 7) carry no integer argument, so they are exempt.
    fn check_minimal(&self, major: u8, info: u8, value: u64) -> Result<(), DecodeError> {
        if !self.config.strict || major == MAJOR_SIMPLE {
            return Ok(());
        }
        let min: u64 = match info {
            24 => 24,
            25 => 0x100,
            26 => 0x1_0000,
            _ => 0x1_0000_0000,
        };
        if value < min {
            Err(DecodeError::NonCanonical)
        } else {
            Ok(())
        }
    }

    /// Reads a head that must have major type `major` and a definite argument.
    fn read_value_of(&mut self, major: u8) -> Result<u64, DecodeError> {
        match self.read_head()? {
            (m, Arg::Value(v)) if m == major => Ok(v),
            (m, Arg::Indefinite) if m == major => Err(DecodeError::IndefiniteLength),
            _ => Err(DecodeError::TypeMismatch),
        }
    }
}

/// Types that can be read from CBOR, borrowing from the input where possible.
pub trait Decode<'a>: Sized {
    /// Reads one value from `decoder`.
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError>;
}

macro_rules! impl_decode_unsigned {
    ($($t:ty),*) => {$(
        impl<'a> Decode<'a> for $t {
            fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
                let v = decoder.decode_u64()?;
                <$t>::try_from(v).map_err(|_| DecodeError::Overflow)
            }
        }
    )*};
}
impl_decode_unsigned!(u8, u16, u32, u64);

macro_rules! impl_decode_signed {
    ($($t:ty),*) => {$(
        impl<'a> Decode<'a> for $t {
            fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
                let v = decoder.decode_i64()?;
                <$t>::try_from(v).map_err(|_| DecodeError::Overflow)
            }
        }
    )*};
}
impl_decode_signed!(i8, i16, i32, i64);

impl<'a> Decode<'a> for bool {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_bool()
    }
}

impl<'a> Decode<'a> for usize {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let v = decoder.decode_u64()?;
        usize::try_from(v).map_err(|_| DecodeError::Overflow)
    }
}

impl<'a> Decode<'a> for isize {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let v = decoder.decode_i64()?;
        isize::try_from(v).map_err(|_| DecodeError::Overflow)
    }
}

impl<'a> Decode<'a> for char {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let text = decoder.decode_str()?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(DecodeError::Malformed),
        }
    }
}

impl<'a> Decode<'a> for () {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_null()
    }
}

impl<'a> Decode<'a> for f64 {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_f64()
    }
}

impl<'a> Decode<'a> for f32 {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let v = decoder.decode_f64()?;
        let narrowed = v as f32;
        // Accept only values that survive the narrowing exactly. NaN is allowed.
        if v.is_nan() || f64::from(narrowed) == v {
            Ok(narrowed)
        } else {
            Err(DecodeError::Overflow)
        }
    }
}

impl<'a> Decode<'a> for &'a str {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_str()
    }
}

impl<'a> Decode<'a> for &'a [u8] {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_bytes()
    }
}

#[cfg(feature = "alloc")]
impl<'a> Decode<'a> for alloc::borrow::Cow<'a, str> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_str().map(alloc::borrow::Cow::Borrowed)
    }
}

#[cfg(feature = "alloc")]
impl<'a> Decode<'a> for alloc::borrow::Cow<'a, [u8]> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_bytes().map(alloc::borrow::Cow::Borrowed)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Decode<'a>> Decode<'a> for alloc::boxed::Box<T> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        T::decode(decoder).map(alloc::boxed::Box::new)
    }
}

/// Decodes a map's pairs and hands each one to `insert`. `insert` returns
/// `false` for a key it already holds, which is reported as a duplicate.
/// Strict mode also checks key order.
#[cfg(feature = "alloc")]
fn decode_pairs<'a, K, V>(
    decoder: &mut Decoder<'a>,
    mut insert: impl FnMut(K, V) -> bool,
) -> Result<(), DecodeError>
where
    K: Decode<'a>,
    V: Decode<'a>,
{
    let n = decoder.decode_map_len()?;
    decoder.enter_container()?;
    let mut prev: Option<(usize, usize)> = None;
    for _ in 0..n {
        let start = decoder.position();
        let key = K::decode(decoder)?;
        let span = (start, decoder.position());
        decoder.check_key_order(prev, span)?;
        prev = Some(span);
        let value = V::decode(decoder)?;
        if !insert(key, value) {
            return Err(DecodeError::DuplicateKey);
        }
    }
    decoder.leave_container();
    Ok(())
}

#[cfg(feature = "alloc")]
impl<'a, K: Decode<'a> + Ord, V: Decode<'a>> Decode<'a> for alloc::collections::BTreeMap<K, V> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let mut out = alloc::collections::BTreeMap::new();
        decode_pairs(decoder, |k, v| out.insert(k, v).is_none())?;
        Ok(out)
    }
}

#[cfg(feature = "std")]
impl<'a, K, V> Decode<'a> for std::collections::HashMap<K, V>
where
    K: Decode<'a> + Eq + std::hash::Hash,
    V: Decode<'a>,
{
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let mut out = std::collections::HashMap::new();
        decode_pairs(decoder, |k, v| out.insert(k, v).is_none())?;
        Ok(out)
    }
}

#[cfg(feature = "alloc")]
impl<'a> Decode<'a> for String {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        decoder.decode_str().map(String::from)
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let n = decoder.decode_array_len()?;
        decoder.enter_container()?;
        let mut out = Vec::with_capacity(n.min(PREALLOC_LIMIT));
        for _ in 0..n {
            out.push(T::decode(decoder)?);
        }
        decoder.leave_container();
        Ok(out)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Option<T> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        if decoder.peek_kind()? == Kind::Null {
            decoder.decode_null()?;
            Ok(None)
        } else {
            T::decode(decoder).map(Some)
        }
    }
}

// Gated on `alloc` because the decode buffer is a `Vec`. Stable Rust has no
// safe way to fill a `[T; N]` one fallible element at a time without either
// `unsafe` (forbidden in this crate) or a `Default` bound on `T`. The encoder
// for `[T; N]` has no such limit and is available without `alloc`.
#[cfg(feature = "alloc")]
impl<'a, T: Decode<'a>, const N: usize> Decode<'a> for [T; N] {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let n = decoder.decode_array_len()?;
        if n != N {
            return Err(DecodeError::TypeMismatch);
        }
        decoder.enter_container()?;
        let mut items = Vec::with_capacity(N.min(PREALLOC_LIMIT));
        for _ in 0..N {
            items.push(T::decode(decoder)?);
        }
        decoder.leave_container();
        // Length was checked above, so this conversion cannot fail.
        <[T; N]>::try_from(items).map_err(|_| DecodeError::TypeMismatch)
    }
}

macro_rules! impl_decode_tuple {
    ($len:expr; $($name:ident),+) => {
        impl<'a, $($name: Decode<'a>),+> Decode<'a> for ($($name,)+) {
            fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
                let n = decoder.decode_array_len()?;
                if n != $len {
                    return Err(DecodeError::TypeMismatch);
                }
                decoder.enter_container()?;
                let value = ( $( $name::decode(decoder)?, )+ );
                decoder.leave_container();
                Ok(value)
            }
        }
    };
}
impl_decode_tuple!(2; A, B);
impl_decode_tuple!(3; A, B, C);
impl_decode_tuple!(4; A, B, C, D);
impl_decode_tuple!(5; A, B, C, D, E);
impl_decode_tuple!(6; A, B, C, D, E, F);
impl_decode_tuple!(7; A, B, C, D, E, F, G);
impl_decode_tuple!(8; A, B, C, D, E, F, G, H);
