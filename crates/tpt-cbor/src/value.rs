//! Dynamically typed CBOR values.

use alloc::vec::Vec;

use crate::decode::{Decode, Decoder, Kind};
use crate::encode::Encoder;
use crate::error::{DecodeError, EncodeError};
use crate::Encode;

/// A schemaless CBOR value that borrows text and byte strings from the input.
///
/// Integers are limited to `i64`. Values outside that range, tags and
/// `undefined` are rejected when decoding.
///
/// Maps are kept as a `Vec` of pairs. Encoding sorts them into canonical
/// order, and rejects duplicate keys. Decoding in strict mode requires the
/// pairs to already be canonical.
///
/// ```
/// use tpt_cbor::{from_slice, to_vec, Value};
///
/// // Canonical encoding of {"a": 1, "b": [2, 3]}
/// let value = Value::Map(vec![
///     (Value::Text("a"), Value::Integer(1)),
///     (Value::Text("b"), Value::Array(vec![Value::Integer(2), Value::Integer(3)])),
/// ]);
/// let bytes = to_vec(&value).unwrap();
/// assert_eq!(bytes, [0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0x82, 0x02, 0x03]);
///
/// // Zero-copy decode: the text borrows from `bytes`.
/// let decoded: Value<'_> = from_slice(&bytes).unwrap();
/// assert_eq!(decoded, value);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// Signed integer in the `i64` range.
    Integer(i64),
    /// Byte string.
    Bytes(&'a [u8]),
    /// UTF-8 text string.
    Text(&'a str),
    /// Definite-length array.
    Array(Vec<Value<'a>>),
    /// Map as ordered key/value pairs.
    Map(Vec<(Value<'a>, Value<'a>)>),
    /// Floating-point number.
    Float(f64),
    /// Boolean.
    Bool(bool),
    /// `null`.
    Null,
}

impl<'a> Value<'a> {
    /// Returns the integer, if this is an `Integer`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the float, if this is a `Float`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the boolean, if this is a `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the text, if this is a `Text`.
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Self::Text(s) => Some(*s),
            _ => None,
        }
    }

    /// Returns the bytes, if this is a `Bytes`.
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Self::Bytes(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the items, if this is an `Array`.
    pub fn as_array(&self) -> Option<&[Value<'a>]> {
        match self {
            Self::Array(items) => Some(items.as_slice()),
            _ => None,
        }
    }

    /// Returns the pairs, if this is a `Map`.
    pub fn as_map(&self) -> Option<&[(Value<'a>, Value<'a>)]> {
        match self {
            Self::Map(pairs) => Some(pairs.as_slice()),
            _ => None,
        }
    }

    /// `true` if this is `Null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Looks up the value for `key` in a map. Returns `None` if this is not a
    /// map or the key is absent.
    pub fn get(&self, key: &Value<'a>) -> Option<&Value<'a>> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Looks up a text key in a map.
    pub fn get_text(&self, key: &str) -> Option<&Value<'a>> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .map(|(_, v)| v)
    }
}

impl Encode for Value<'_> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        match self {
            Value::Integer(i) => encoder.encode_i64(*i),
            Value::Bytes(b) => encoder.encode_bytes(b),
            Value::Text(t) => encoder.encode_str(t),
            Value::Float(f) => encoder.encode_f64(*f),
            Value::Bool(b) => encoder.encode_bool(*b),
            Value::Null => encoder.encode_null(),
            Value::Array(items) => {
                encoder.enter_container()?;
                encoder.encode_array_header(items.len())?;
                for item in items {
                    item.encode(encoder)?;
                }
                encoder.leave_container();
                Ok(())
            }
            Value::Map(pairs) => {
                encoder.enter_container()?;
                // Encode each key once, then sort by its encoded bytes. The
                // encoded form is the sort key, as RFC 8949 §4.2.1 requires.
                let mut keyed: Vec<(Vec<u8>, &Value<'_>)> = Vec::with_capacity(pairs.len());
                for (key, value) in pairs {
                    let mut key_bytes = Vec::new();
                    {
                        let mut key_encoder = encoder.nested(&mut key_bytes);
                        key.encode(&mut key_encoder)?;
                    }
                    keyed.push((key_bytes, value));
                }
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
                for (a, b) in keyed.iter().zip(keyed.iter().skip(1)) {
                    if a.0 == b.0 {
                        return Err(EncodeError::DuplicateKey);
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
        }
    }
}

impl<'a> Decode<'a> for Value<'a> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        match decoder.peek_kind()? {
            Kind::Unsigned | Kind::Negative => decoder.decode_i64().map(Value::Integer),
            Kind::Bytes => decoder.decode_bytes().map(Value::Bytes),
            Kind::Text => decoder.decode_str().map(Value::Text),
            Kind::Bool => decoder.decode_bool().map(Value::Bool),
            Kind::Null => {
                decoder.decode_null()?;
                Ok(Value::Null)
            }
            Kind::Float => decoder.decode_f64().map(Value::Float),
            Kind::Array => {
                let n = decoder.decode_array_len()?;
                decoder.enter_container()?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(Value::decode(decoder)?);
                }
                decoder.leave_container();
                Ok(Value::Array(items))
            }
            Kind::Map => {
                let n = decoder.decode_map_len()?;
                decoder.enter_container()?;
                let mut pairs = Vec::with_capacity(n);
                let mut prev: Option<(usize, usize)> = None;
                for _ in 0..n {
                    let key_start = decoder.position();
                    let key = Value::decode(decoder)?;
                    let key_span = (key_start, decoder.position());
                    decoder.check_key_order(prev, key_span)?;
                    prev = Some(key_span);
                    let value = Value::decode(decoder)?;
                    pairs.push((key, value));
                }
                decoder.leave_container();
                Ok(Value::Map(pairs))
            }
        }
    }
}
