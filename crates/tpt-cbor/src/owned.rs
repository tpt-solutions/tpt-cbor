//! Owned dynamic CBOR values.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::decode::{Decode, Decoder, Kind, PREALLOC_LIMIT};
use crate::encode::{encode_pairs, Encode, Encoder};
use crate::error::{DecodeError, EncodeError};
use crate::value::{write_float, write_text, Value};

/// An owned, schemaless CBOR value.
///
/// It has the same shape as [`Value`], but text and byte strings are owned,
/// so it has no lifetime. Use it when a value must outlive its input, or when
/// it is built at run time from owned data. Convert a borrowed value with
/// [`Value::into_owned`].
///
/// Integers are limited to `i64`, and tags and `undefined` are rejected, as
/// for [`Value`].
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedValue {
    /// Signed integer in the `i64` range.
    Integer(i64),
    /// Byte string.
    Bytes(Vec<u8>),
    /// UTF-8 text string.
    Text(String),
    /// Definite-length array.
    Array(Vec<OwnedValue>),
    /// Map as ordered key/value pairs.
    Map(Vec<(OwnedValue, OwnedValue)>),
    /// Floating-point number.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// `null`.
    Null,
}

impl OwnedValue {
    /// Returns the integer, if this is an `Integer`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the text, if this is a `Text`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Looks up a text key in a map.
    pub fn get_text(&self, key: &str) -> Option<&OwnedValue> {
        match self {
            Self::Map(pairs) => pairs
                .iter()
                .find(|(k, _)| k.as_str() == Some(key))
                .map(|(_, v)| v),
            _ => None,
        }
    }
}

impl Value<'_> {
    /// Copies the borrowed text and bytes so the result does not depend on the
    /// input.
    pub fn into_owned(self) -> OwnedValue {
        match self {
            Value::Integer(i) => OwnedValue::Integer(i),
            Value::Bytes(b) => OwnedValue::Bytes(b.to_vec()),
            Value::Text(t) => OwnedValue::Text(String::from(t)),
            Value::Array(items) => {
                OwnedValue::Array(items.into_iter().map(Value::into_owned).collect())
            }
            Value::Map(pairs) => OwnedValue::Map(
                pairs
                    .into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
            ),
            Value::Float(f) => OwnedValue::Float(f),
            Value::Bool(b) => OwnedValue::Bool(b),
            Value::Null => OwnedValue::Null,
        }
    }
}

impl Encode for OwnedValue {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        match self {
            OwnedValue::Integer(i) => encoder.encode_i64(*i),
            OwnedValue::Bytes(b) => encoder.encode_bytes(b),
            OwnedValue::Text(t) => encoder.encode_str(t),
            OwnedValue::Float(f) => encoder.encode_f64(*f),
            OwnedValue::Bool(b) => encoder.encode_bool(*b),
            OwnedValue::Null => encoder.encode_null(),
            OwnedValue::Array(items) => {
                encoder.enter_container()?;
                encoder.encode_array_header(items.len())?;
                for item in items {
                    item.encode(encoder)?;
                }
                encoder.leave_container();
                Ok(())
            }
            OwnedValue::Map(pairs) => {
                encode_pairs(pairs.iter().map(|(k, v)| (k, v)), pairs.len(), encoder)
            }
        }
    }
}

impl<'a> Decode<'a> for OwnedValue {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        match decoder.peek_kind()? {
            Kind::Unsigned | Kind::Negative => decoder.decode_i64().map(OwnedValue::Integer),
            Kind::Bytes => decoder
                .decode_bytes()
                .map(|b| OwnedValue::Bytes(b.to_vec())),
            Kind::Text => decoder
                .decode_str()
                .map(|s| OwnedValue::Text(String::from(s))),
            Kind::Bool => decoder.decode_bool().map(OwnedValue::Bool),
            Kind::Null => {
                decoder.decode_null()?;
                Ok(OwnedValue::Null)
            }
            Kind::Float => decoder.decode_f64().map(OwnedValue::Float),
            Kind::Array => {
                let n = decoder.decode_array_len()?;
                decoder.enter_container()?;
                let mut items = Vec::with_capacity(n.min(PREALLOC_LIMIT));
                for _ in 0..n {
                    items.push(OwnedValue::decode(decoder)?);
                }
                decoder.leave_container();
                Ok(OwnedValue::Array(items))
            }
            Kind::Map => {
                let n = decoder.decode_map_len()?;
                decoder.enter_container()?;
                let mut pairs = Vec::with_capacity(n.min(PREALLOC_LIMIT));
                let mut prev: Option<(usize, usize)> = None;
                for _ in 0..n {
                    let start = decoder.position();
                    let key = OwnedValue::decode(decoder)?;
                    let span = (start, decoder.position());
                    decoder.check_key_order(prev, span)?;
                    prev = Some(span);
                    let value = OwnedValue::decode(decoder)?;
                    pairs.push((key, value));
                }
                decoder.leave_container();
                Ok(OwnedValue::Map(pairs))
            }
        }
    }
}

/// Diagnostic notation (RFC 8949 §8), as for [`Value`].
impl fmt::Display for OwnedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnedValue::Integer(i) => write!(f, "{i}"),
            OwnedValue::Bytes(b) => {
                f.write_str("h'")?;
                for byte in b {
                    write!(f, "{byte:02x}")?;
                }
                f.write_str("'")
            }
            OwnedValue::Text(t) => write_text(f, t),
            OwnedValue::Float(x) => write_float(f, *x),
            OwnedValue::Bool(b) => write!(f, "{b}"),
            OwnedValue::Null => f.write_str("null"),
            OwnedValue::Array(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            OwnedValue::Map(pairs) => {
                f.write_str("{")?;
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                f.write_str("}")
            }
        }
    }
}

impl From<i64> for OwnedValue {
    fn from(v: i64) -> Self {
        OwnedValue::Integer(v)
    }
}

impl From<bool> for OwnedValue {
    fn from(v: bool) -> Self {
        OwnedValue::Bool(v)
    }
}

impl From<f64> for OwnedValue {
    fn from(v: f64) -> Self {
        OwnedValue::Float(v)
    }
}

impl From<&str> for OwnedValue {
    fn from(v: &str) -> Self {
        OwnedValue::Text(String::from(v))
    }
}

impl From<String> for OwnedValue {
    fn from(v: String) -> Self {
        OwnedValue::Text(v)
    }
}
