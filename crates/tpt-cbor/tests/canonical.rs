#![cfg(feature = "alloc")]
//! Canonical-form rules: encoder output, strict rejection, lenient acceptance.

use tpt_cbor::{from_slice, from_slice_with_config, to_vec, Config, DecodeError, Decoder, Value};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

#[test]
fn map_keys_sorted_bytewise_not_by_length() {
    // Encoded keys: 0x1000 -> 19 10 00 (3 bytes), [] -> 80 (1 byte).
    // RFC 8949 §4.2.1 bytewise order puts 19 10 00 first, even though it is
    // longer. Length-first ordering would put [] first. The spec was amended
    // to bytewise, so this test pins that choice.
    let value = Value::Map(vec![
        (Value::Array(vec![]), Value::Integer(1)),
        (Value::Integer(4096), Value::Integer(2)),
    ]);
    let bytes = to_vec(&value).unwrap();
    assert_eq!(bytes, hex("a2 19 10 00 02 80 01"));
    // The bytes are strictly decodable, which proves they are canonically ordered.
    assert_eq!(
        from_slice::<Value<'_>>(&bytes)
            .unwrap()
            .as_map()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn map_encoding_is_deterministic_regardless_of_insertion_order() {
    let a = Value::Map(vec![
        (Value::Text("zeta"), Value::Integer(1)),
        (Value::Text("alpha"), Value::Integer(2)),
        (Value::Integer(10), Value::Integer(3)),
    ]);
    let b = Value::Map(vec![
        (Value::Integer(10), Value::Integer(3)),
        (Value::Text("alpha"), Value::Integer(2)),
        (Value::Text("zeta"), Value::Integer(1)),
    ]);
    assert_eq!(to_vec(&a).unwrap(), to_vec(&b).unwrap());
}

#[test]
fn encoding_rejects_duplicate_keys() {
    let value = Value::Map(vec![
        (Value::Text("k"), Value::Integer(1)),
        (Value::Text("k"), Value::Integer(2)),
    ]);
    assert_eq!(to_vec(&value), Err(tpt_cbor::EncodeError::DuplicateKey));
}

#[test]
fn encoding_is_deterministic_for_equal_values() {
    let value = Value::Array(vec![Value::Float(0.5), Value::Integer(-300)]);
    assert_eq!(to_vec(&value).unwrap(), to_vec(&value.clone()).unwrap());
}

#[test]
fn strict_rejects_non_minimal_integers() {
    // 1 encoded with a one-byte argument.
    assert_eq!(
        from_slice::<u8>(&hex("18 01")),
        Err(DecodeError::NonCanonical)
    );
    // 255 encoded with a two-byte argument.
    assert_eq!(
        from_slice::<u16>(&hex("19 00 ff")),
        Err(DecodeError::NonCanonical)
    );
    // 65535 encoded with a four-byte argument.
    assert_eq!(
        from_slice::<u32>(&hex("1a 00 00 ff ff")),
        Err(DecodeError::NonCanonical)
    );
    // 4294967295 encoded with an eight-byte argument.
    assert_eq!(
        from_slice::<u64>(&hex("1b 00 00 00 00 ff ff ff ff")),
        Err(DecodeError::NonCanonical)
    );
    // Same rule for negative integers.
    assert_eq!(
        from_slice::<i64>(&hex("38 01")),
        Err(DecodeError::NonCanonical)
    );
}

#[test]
fn lenient_accepts_non_minimal_integers() {
    let v: u8 = from_slice_with_config(&hex("18 01"), Config::lenient()).unwrap();
    assert_eq!(v, 1);
    let v: u64 =
        from_slice_with_config(&hex("1b 00 00 00 00 00 00 00 05"), Config::lenient()).unwrap();
    assert_eq!(v, 5);
}

#[test]
fn strict_rejects_non_canonical_floats() {
    // 1.0 as double: must be half.
    assert_eq!(
        from_slice::<f64>(&hex("fb 3f f0 00 00 00 00 00 00")),
        Err(DecodeError::NonCanonical)
    );
    // 1.0 as single: must be half.
    assert_eq!(
        from_slice::<f32>(&hex("fa 3f 80 00 00")),
        Err(DecodeError::NonCanonical)
    );
    // NaN with a non-canonical payload (0x7c01 instead of 0x7e00).
    assert_eq!(
        from_slice::<f64>(&hex("f9 7c 01")),
        Err(DecodeError::NonCanonical)
    );
    // NaN in double form.
    assert_eq!(
        from_slice::<f64>(&hex("fb 7f f8 00 00 00 00 00 00")),
        Err(DecodeError::NonCanonical)
    );
    // -0.0 as double: must be half 0x8000.
    assert_eq!(
        from_slice::<f64>(&hex("fb 80 00 00 00 00 00 00 00")),
        Err(DecodeError::NonCanonical)
    );
}

#[test]
fn lenient_accepts_non_canonical_floats() {
    let v: f64 =
        from_slice_with_config(&hex("fb 3f f0 00 00 00 00 00 00"), Config::lenient()).unwrap();
    assert_eq!(v, 1.0);
    let v: f64 = from_slice_with_config(&hex("f9 7c 01"), Config::lenient()).unwrap();
    assert!(v.is_nan());
}

#[test]
fn strict_rejects_unsorted_and_duplicate_keys() {
    // {"b": 1, "a": 2}: keys out of order.
    let unsorted = hex("a2 61 62 01 61 61 02");
    assert_eq!(
        from_slice::<Value<'_>>(&unsorted),
        Err(DecodeError::UnsortedMap)
    );
    // {"a": 1, "a": 2}: duplicate.
    let dup = hex("a2 61 61 01 61 61 02");
    assert_eq!(
        from_slice::<Value<'_>>(&dup),
        Err(DecodeError::DuplicateKey)
    );

    // Same checks through the zero-allocation skip path.
    let mut d = Decoder::new(&unsorted);
    assert_eq!(d.skip_item(), Err(DecodeError::UnsortedMap));
    let mut d = Decoder::new(&dup);
    assert_eq!(d.skip_item(), Err(DecodeError::DuplicateKey));
}

#[test]
fn lenient_accepts_unsorted_and_duplicate_keys() {
    let unsorted = hex("a2 61 62 01 61 61 02");
    let v: Value<'_> = from_slice_with_config(&unsorted, Config::lenient()).unwrap();
    assert_eq!(v.as_map().unwrap().len(), 2);
    let dup = hex("a2 61 61 01 61 61 02");
    let v: Value<'_> = from_slice_with_config(&dup, Config::lenient()).unwrap();
    assert_eq!(v.as_map().unwrap().len(), 2);
}

#[test]
fn value_accessors_and_lookup() {
    let bytes = hex("a2 61 61 01 61 62 82 02 03");
    let v: Value<'_> = from_slice(&bytes).unwrap();
    assert_eq!(v.get_text("a").and_then(Value::as_i64), Some(1));
    assert_eq!(
        v.get_text("b")
            .and_then(Value::as_array)
            .map(<[Value<'_>]>::len),
        Some(2)
    );
    assert_eq!(v.get_text("missing"), None);
    assert_eq!(Value::Text("x").as_i64(), None);
    assert!(Value::Null.is_null());
}

#[test]
fn map_values_can_hold_mixed_key_types() {
    // Keys of different major types still sort deterministically.
    let v = Value::Map(vec![
        (Value::Bool(true), Value::Null),
        (Value::Text("t"), Value::Null),
        (Value::Integer(-1), Value::Null),
    ]);
    let bytes = to_vec(&v).unwrap();
    let back: Value<'_> = from_slice(&bytes).unwrap();
    assert_eq!(back.as_map().unwrap().len(), 3);
}
