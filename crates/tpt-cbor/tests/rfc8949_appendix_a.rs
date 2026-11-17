#![cfg(feature = "alloc")]
//! RFC 8949 Appendix A examples, transcribed from the RFC.
//!
//! Each row is checked against the behaviour this crate promises:
//!
//! * Canonical rows decode, and re-encoding the decoded value gives the same
//!   bytes.
//! * Non-canonical float rows are rejected in strict mode with
//!   `NonCanonical`, and accepted in lenient mode.
//! * Rows with tags, `undefined` or unassigned simple values are rejected with
//!   `UnsupportedType`, as documented.
//! * Rows with indefinite lengths are rejected with `IndefiniteLength` in both
//!   modes.
//! * Integers outside `i64` are rejected by `Value` with `Overflow`.

use tpt_cbor::{from_slice, from_slice_with_config, to_vec, Config, DecodeError, Value};

/// Parses a compact hex string such as `"1903e8"`.
fn h(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length: {s}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex byte"))
        .collect()
}

/// Decodes `bytes` as a `Value`, re-encodes it, and checks the bytes match.
fn value_round_trips(bytes: &[u8]) -> Value<'_> {
    let v: Value<'_> = from_slice(bytes).unwrap_or_else(|e| panic!("{bytes:02x?}: {e:?}"));
    assert_eq!(to_vec(&v).unwrap(), bytes, "re-encode of {bytes:02x?}");
    v
}

#[test]
fn integers_round_trip() {
    let rows: &[(&str, i64)] = &[
        ("00", 0),
        ("01", 1),
        ("0a", 10),
        ("17", 23),
        ("1818", 24),
        ("1819", 25),
        ("1864", 100),
        ("1903e8", 1000),
        ("1a000f4240", 1_000_000),
        ("1b000000e8d4a51000", 1_000_000_000_000),
        ("20", -1),
        ("29", -10),
        ("3863", -100),
        ("3903e7", -1000),
    ];
    for (hex, expected) in rows {
        let bytes = h(hex);
        assert_eq!(
            value_round_trips(&bytes),
            Value::Integer(*expected),
            "{hex}"
        );
        assert_eq!(from_slice::<i64>(&bytes).unwrap(), *expected, "{hex}");
    }
}

#[test]
fn integers_beyond_i64_are_rejected_by_value() {
    // 2^64 - 1 is a valid CBOR unsigned integer but does not fit `i64`.
    let max = h("1bffffffffffffffff");
    assert_eq!(from_slice::<u64>(&max).unwrap(), u64::MAX);
    assert_eq!(from_slice::<Value<'_>>(&max), Err(DecodeError::Overflow));
    // -2^64 is a valid CBOR negative integer, also outside `i64`.
    let min = h("3bffffffffffffffff");
    assert_eq!(from_slice::<Value<'_>>(&min), Err(DecodeError::Overflow));
}

#[test]
fn bignum_tags_are_unsupported() {
    // Tags 2 and 3 carry bignums. Tags are outside the supported subset.
    for hex in ["c249010000000000000000", "c349010000000000000000"] {
        assert_eq!(
            from_slice::<Value<'_>>(&h(hex)),
            Err(DecodeError::UnsupportedType),
            "{hex}"
        );
    }
}

#[test]
fn canonical_floats_round_trip_at_smallest_width() {
    // (encoding, expected value). Each encoding is the shortest lossless form.
    let rows: &[(&str, f64)] = &[
        ("f90000", 0.0),
        ("f98000", -0.0),
        ("f93c00", 1.0),
        ("fb3ff199999999999a", 1.1),
        ("f93e00", 1.5),
        ("f97bff", 65504.0),
        ("fa47c35000", 100000.0),
        ("fa7f7fffff", f64::from(f32::MAX)),
        ("fb7e37e43c8800759c", 1.0e+300),
        ("f90001", 5.960464477539063e-8),
        ("f90400", 0.00006103515625),
        ("f9c400", -4.0),
        ("fbc010666666666666", -4.1),
        ("f97c00", f64::INFINITY),
        ("f9fc00", f64::NEG_INFINITY),
    ];
    for (hex, expected) in rows {
        let bytes = h(hex);
        let got = from_slice::<f64>(&bytes).unwrap_or_else(|e| panic!("{hex}: {e:?}"));
        assert_eq!(got.to_bits(), expected.to_bits(), "decode {hex}");
        assert_eq!(to_vec(&got).unwrap(), bytes, "encode {hex}");
    }
}

#[test]
fn canonical_nan_round_trips() {
    let bytes = h("f97e00");
    let got = from_slice::<f64>(&bytes).unwrap();
    assert!(got.is_nan());
    assert_eq!(to_vec(&got).unwrap(), bytes);
}

#[test]
fn non_canonical_special_floats_are_rejected_in_strict_mode() {
    // These are valid encodings of the same values, but not the shortest form.
    let rows = [
        "fa7f800000",         // +Infinity as single
        "fa7fc00000",         // NaN as single
        "faff800000",         // -Infinity as single
        "fb7ff0000000000000", // +Infinity as double
        "fb7ff8000000000000", // NaN as double
        "fbfff0000000000000", // -Infinity as double
    ];
    for hex in rows {
        let bytes = h(hex);
        assert_eq!(
            from_slice::<f64>(&bytes),
            Err(DecodeError::NonCanonical),
            "strict {hex}"
        );
        // Lenient mode accepts them: the value is infinite or NaN, as encoded.
        let got = from_slice_with_config::<f64>(&bytes, Config::lenient())
            .unwrap_or_else(|e| panic!("lenient {hex}: {e:?}"));
        assert!(got.is_nan() || got.is_infinite(), "lenient {hex}");
    }
}

#[test]
fn simple_values_round_trip() {
    assert_eq!(value_round_trips(&h("f4")), Value::Bool(false));
    assert_eq!(value_round_trips(&h("f5")), Value::Bool(true));
    assert_eq!(value_round_trips(&h("f6")), Value::Null);
}

#[test]
fn undefined_and_unassigned_simple_values_are_unsupported() {
    // undefined, simple(16), simple(255)
    for hex in ["f7", "f0", "f8ff"] {
        assert_eq!(
            from_slice::<Value<'_>>(&h(hex)),
            Err(DecodeError::UnsupportedType),
            "{hex}"
        );
    }
}

#[test]
fn tagged_items_are_unsupported() {
    // Date strings, epoch times, base64 hints, and URIs: all tagged.
    let rows = [
        "c074323031332d30332d32315432303a30343a30305a",
        "c11a514b67b0",
        "c1fb41d452d9ec200000",
        "d74401020304",
        "d818456449455446",
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
    ];
    for hex in rows {
        assert_eq!(
            from_slice::<Value<'_>>(&h(hex)),
            Err(DecodeError::UnsupportedType),
            "{hex}"
        );
    }
}

#[test]
fn byte_and_text_strings_round_trip() {
    let empty_bytes = h("40");
    assert_eq!(value_round_trips(&empty_bytes), Value::Bytes(&[]));
    let bytes = h("4401020304");
    assert_eq!(value_round_trips(&bytes), Value::Bytes(&[1, 2, 3, 4]));

    let rows: &[(&str, &str)] = &[
        ("60", ""),
        ("6161", "a"),
        ("6449455446", "IETF"),
        ("62225c", "\"\\"),
        ("62c3bc", "\u{fc}"),
        ("63e6b0b4", "\u{6c34}"),
        ("64f0908591", "\u{10151}"),
    ];
    for (hex, expected) in rows {
        let bytes = h(hex);
        assert_eq!(value_round_trips(&bytes), Value::Text(expected), "{hex}");
    }
}

#[test]
fn arrays_and_maps_round_trip() {
    assert_eq!(value_round_trips(&h("80")), Value::Array(vec![]));
    assert_eq!(
        value_round_trips(&h("83010203")),
        Value::Array(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(3)
        ])
    );
    assert_eq!(
        value_round_trips(&h("8301820203820405")),
        Value::Array(vec![
            Value::Integer(1),
            Value::Array(vec![Value::Integer(2), Value::Integer(3)]),
            Value::Array(vec![Value::Integer(4), Value::Integer(5)]),
        ])
    );
    // 25 elements: the length needs the one-byte argument form.
    let long = h("98190102030405060708090a0b0c0d0e0f101112131415161718181819");
    let v = value_round_trips(&long);
    assert_eq!(v.as_array().unwrap().len(), 25);
    assert_eq!(v.as_array().unwrap()[24], Value::Integer(25));

    assert_eq!(value_round_trips(&h("a0")), Value::Map(vec![]));
    assert_eq!(
        value_round_trips(&h("a201020304")),
        Value::Map(vec![
            (Value::Integer(1), Value::Integer(2)),
            (Value::Integer(3), Value::Integer(4)),
        ])
    );
    assert_eq!(
        value_round_trips(&h("a26161016162820203")),
        Value::Map(vec![
            (Value::Text("a"), Value::Integer(1)),
            (
                Value::Text("b"),
                Value::Array(vec![Value::Integer(2), Value::Integer(3)])
            ),
        ])
    );
    assert_eq!(
        value_round_trips(&h("826161a161626163")),
        Value::Array(vec![
            Value::Text("a"),
            Value::Map(vec![(Value::Text("b"), Value::Text("c"))]),
        ])
    );
    let five_bytes = h("a56161614161626142616361436164614461656145");
    let five = value_round_trips(&five_bytes);
    assert_eq!(five.as_map().unwrap().len(), 5);
    assert_eq!(five.get_text("e"), Some(&Value::Text("E")));
}

#[test]
fn indefinite_length_items_are_always_rejected() {
    // Every indefinite-length row of Appendix A.
    let rows = [
        "5f42010243030405ff",
        "7f657374726561646d696e67ff",
        "9fff",
        "9f018202039f0405ffff",
        "9f01820203820405ff",
        "83018202039f0405ff",
        "83019f0203ff820405",
        "9f0102030405060708090a0b0c0d0e0f101112131415161718181819ff",
        "bf61610161629f0203ffff",
        "826161bf61626163ff",
        "bf6346756ef563416d7421ff",
    ];
    for hex in rows {
        let bytes = h(hex);
        assert_eq!(
            from_slice::<Value<'_>>(&bytes),
            Err(DecodeError::IndefiniteLength),
            "strict {hex}"
        );
        assert_eq!(
            from_slice_with_config::<Value<'_>>(&bytes, Config::lenient()),
            Err(DecodeError::IndefiniteLength),
            "lenient {hex}"
        );
    }
}
