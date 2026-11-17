#![cfg(feature = "alloc")]
//! Phase 7 behaviour: conversions, indexing, diagnostic notation, the `cbor!`
//! macro, owned values, byte strings, extra codec impls, error offsets,
//! sequences and whole-input helpers.

use std::borrow::Cow;
use std::collections::BTreeMap;

use tpt_cbor::prelude::*;
use tpt_cbor::{
    canonicalize, from_slice, from_slice_located, from_slice_with_config, is_canonical, sequence,
    validate, ByteBuf, Config, DecodeError, Located, OwnedValue, Value,
};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

#[test]
fn value_conversions() {
    assert_eq!(Value::from(5u8), Value::Integer(5));
    assert_eq!(Value::from(-7i32), Value::Integer(-7));
    assert_eq!(Value::from(1.5f32), Value::Float(1.5));
    assert_eq!(Value::from(true), Value::Bool(true));
    assert_eq!(Value::from("x"), Value::Text("x"));
    assert_eq!(Value::from(&[1u8, 2][..]), Value::Bytes(&[1, 2]));
    assert_eq!(Value::from(Some("x")), Value::Text("x"));
    assert_eq!(Value::from(None::<i32>), Value::Null);
}

#[test]
fn indexing_never_panics_and_returns_null_when_absent() {
    let v = cbor!({ "a": 1, "b": [2, 3] });
    assert_eq!(v["a"], Value::Integer(1));
    assert_eq!(v["missing"], Value::Null);
    assert_eq!(v["b"][1], Value::Integer(3));
    assert_eq!(v["b"][9], Value::Null);
    // Indexing a non-map or non-array is also null, not a panic.
    assert_eq!(Value::Integer(1)["a"], Value::Null);
    assert_eq!(Value::Integer(1)[0], Value::Null);
}

#[test]
fn diagnostic_notation_display() {
    let v = cbor!({ "a": 1, "b": [2, 3] });
    assert_eq!(v.to_string(), r#"{"a": 1, "b": [2, 3]}"#);
    assert_eq!(Value::Bytes(&[0xde, 0xad]).to_string(), "h'dead'");
    assert_eq!(Value::Float(1.5).to_string(), "1.5");
    assert_eq!(Value::Float(1.0).to_string(), "1.0");
    assert_eq!(Value::Float(f64::NAN).to_string(), "NaN");
    assert_eq!(Value::Float(f64::INFINITY).to_string(), "Infinity");
    assert_eq!(Value::Float(f64::NEG_INFINITY).to_string(), "-Infinity");
    assert_eq!(Value::Null.to_string(), "null");
    assert_eq!(Value::Bool(false).to_string(), "false");
    // Quote, backslash and newline are escaped.
    assert_eq!(Value::Text("a\"b\\\n").to_string(), r#""a\"b\\\u000a""#);
}

#[test]
fn cbor_macro_builds_the_same_values_as_by_hand() {
    let built = cbor!({ "a": 1, "b": [2, 3] });
    let by_hand = Value::Map(vec![
        (Value::Text("a"), Value::Integer(1)),
        (
            Value::Text("b"),
            Value::Array(vec![Value::Integer(2), Value::Integer(3)]),
        ),
    ]);
    assert_eq!(built, by_hand);
    assert_eq!(cbor!(null), Value::Null);
    assert_eq!(cbor!(true), Value::Bool(true));
    assert_eq!(cbor!([]), Value::Array(vec![]));
    assert_eq!(cbor!([(-1)]), Value::Array(vec![Value::Integer(-1)]));
}

#[test]
fn owned_value_round_trips_and_matches_borrowed() {
    // {"a": 1, "b": [2, 3]} in canonical form.
    let bytes = hex("a2 61 61 01 61 62 82 02 03");
    let borrowed: Value<'_> = from_slice(&bytes).unwrap();
    let owned: OwnedValue = from_slice(&bytes).unwrap();
    assert_eq!(borrowed.clone().into_owned(), owned);
    assert_eq!(to_vec(&owned).unwrap(), bytes);
    assert_eq!(owned.get_text("a"), Some(&OwnedValue::Integer(1)));
    assert_eq!(owned.to_string(), borrowed.to_string());
}

#[test]
fn byte_buf_is_a_byte_string() {
    let bytes = hex("42 01 02");
    assert_eq!(to_vec(&ByteBuf(vec![1, 2])).unwrap(), bytes);
    assert_eq!(from_slice::<ByteBuf>(&bytes).unwrap(), ByteBuf(vec![1, 2]));
    // Vec<u8> is an array of integers, not a byte string.
    assert_eq!(to_vec(&vec![1u8, 2]).unwrap(), hex("82 01 02"));
}

#[test]
fn extra_scalar_codecs() {
    assert_eq!(to_vec(&5usize).unwrap(), hex("05"));
    assert_eq!(from_slice::<usize>(&hex("18 18")).unwrap(), 24);
    assert_eq!(to_vec(&-3isize).unwrap(), hex("22"));
    assert_eq!(from_slice::<isize>(&hex("22")).unwrap(), -3);
    assert_eq!(to_vec(&'\u{e9}').unwrap(), hex("62 c3 a9"));
    assert_eq!(from_slice::<char>(&hex("62 c3 a9")).unwrap(), '\u{e9}');
    // A text string with two characters is not a char.
    assert_eq!(
        from_slice::<char>(&hex("62 61 62")),
        Err(DecodeError::Malformed)
    );
    assert_eq!(to_vec(&()).unwrap(), hex("f6"));
    assert!(from_slice::<()>(&hex("f6")).is_ok());
}

#[test]
fn box_and_cow_codecs() {
    let boxed: Box<u8> = Box::new(7);
    assert_eq!(to_vec(&boxed).unwrap(), hex("07"));
    assert_eq!(from_slice::<Box<u8>>(&hex("07")).unwrap(), Box::new(7));

    let text: Cow<'_, str> = Cow::Borrowed("hi");
    assert_eq!(to_vec(&text).unwrap(), hex("62 68 69"));
    assert_eq!(from_slice::<Cow<'_, str>>(&hex("62 68 69")).unwrap(), "hi");

    let bytes: Cow<'_, [u8]> = Cow::Borrowed(&[1, 2]);
    assert_eq!(to_vec(&bytes).unwrap(), hex("42 01 02"));
}

#[test]
fn btree_map_encodes_in_canonical_order() {
    let mut map = BTreeMap::new();
    map.insert("b".to_string(), 1u8);
    map.insert("a".to_string(), 2u8);
    // "a" sorts before "b" by encoded bytes.
    let bytes = to_vec(&map).unwrap();
    assert_eq!(bytes, hex("a2 61 61 02 61 62 01"));
    let back: BTreeMap<String, u8> = from_slice(&bytes).unwrap();
    assert_eq!(back, map);
}

#[test]
fn btree_map_decode_rejects_duplicates_and_disorder() {
    // Duplicate key "a".
    assert_eq!(
        from_slice::<BTreeMap<String, u8>>(&hex("a2 61 61 01 61 61 02")),
        Err(DecodeError::DuplicateKey)
    );
    // "b" before "a" is out of canonical order (strict only).
    assert_eq!(
        from_slice::<BTreeMap<String, u8>>(&hex("a2 61 62 01 61 61 02")),
        Err(DecodeError::UnsortedMap)
    );
    let lenient = from_slice_with_config::<BTreeMap<String, u8>>(
        &hex("a2 61 62 01 61 61 02"),
        Config::lenient(),
    )
    .unwrap();
    assert_eq!(lenient.len(), 2);
}

#[test]
fn eight_tuples_round_trip() {
    let value = (1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8);
    let bytes = to_vec(&value).unwrap();
    assert_eq!(bytes, hex("88 01 02 03 04 05 06 07 08"));
    let back: (u8, u8, u8, u8, u8, u8, u8, u8) = from_slice(&bytes).unwrap();
    assert_eq!(back, value);
}

#[test]
fn located_errors_report_an_offset() {
    // 0x18 0x01 is a non-minimal integer; decoding stops after its two bytes.
    assert_eq!(
        from_slice_located::<u64>(&hex("18 01")),
        Err(Located {
            error: DecodeError::NonCanonical,
            offset: 2,
        })
    );
    // Trailing bytes are reported at the end of the first item.
    assert_eq!(
        from_slice_located::<u64>(&hex("01 02")),
        Err(Located {
            error: DecodeError::TrailingBytes,
            offset: 1,
        })
    );
    assert_eq!(from_slice_located::<u64>(&hex("01")), Ok(1));
}

#[test]
fn validate_and_canonicality_helpers() {
    assert!(validate(&hex("01"), Config::strict()).is_ok());
    assert_eq!(
        validate(&hex("01 02"), Config::strict()),
        Err(Located {
            error: DecodeError::TrailingBytes,
            offset: 1,
        })
    );
    assert!(is_canonical(&hex("01")));
    assert!(!is_canonical(&hex("18 01")));
    // Lenient validation accepts the non-minimal integer.
    assert!(validate(&hex("18 01"), Config::lenient()).is_ok());
}

#[test]
fn canonicalize_rewrites_non_canonical_input() {
    assert_eq!(canonicalize(&hex("18 01")).unwrap(), hex("01"));
    // A map with unsorted keys is rewritten in canonical order.
    assert_eq!(
        canonicalize(&hex("a2 61 62 01 61 61 02")).unwrap(),
        hex("a2 61 61 02 61 62 01")
    );
    // Values outside the Value model are rejected, as documented.
    assert_eq!(
        canonicalize(&hex("c1 01")),
        Err(DecodeError::UnsupportedType)
    );
}

#[test]
fn sequence_iterates_concatenated_items() {
    let items: Vec<u8> = sequence(&hex("01 02 03")).map(|r| r.unwrap()).collect();
    assert_eq!(items, vec![1, 2, 3]);

    let truncated = hex("01 18");
    let mut seq = sequence::<u8>(&truncated);
    assert_eq!(seq.next(), Some(Ok(1)));
    assert_eq!(seq.next(), Some(Err(DecodeError::UnexpectedEof)));
    assert_eq!(seq.next(), None);

    assert_eq!(sequence::<u8>(&[]).next(), None);
}

#[cfg(feature = "std")]
mod std_helpers {
    use std::io::{self, Read};

    use tpt_cbor::{from_reader, ReadError};

    use super::*;

    struct Failing;

    impl Read for Failing {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("broken"))
        }
    }

    #[test]
    fn from_reader_decodes_owned_types() {
        let value: OwnedValue = from_reader(&hex("a1 61 61 01")[..]).unwrap();
        assert_eq!(value.get_text("a"), Some(&OwnedValue::Integer(1)));
        assert_eq!(from_reader::<String, _>(&hex("61 61")[..]).unwrap(), "a");
    }

    #[test]
    fn from_reader_separates_io_and_decode_errors() {
        assert_eq!(
            from_reader::<u8, _>(&hex("18")[..]),
            Err(ReadError::Decode(DecodeError::UnexpectedEof))
        );
        assert_eq!(
            from_reader::<u8, _>(Failing),
            Err(ReadError::Io(io::ErrorKind::Other))
        );
    }
}
