#![cfg(feature = "alloc")]
//! Encode/decode vectors. Byte strings follow RFC 8949 Appendix A and are
//! already in canonical form, so each one must round-trip exactly.

use tpt_cbor::{from_slice, to_vec, Value};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

#[test]
fn unsigned_integers() {
    let cases: &[(u64, &str)] = &[
        (0, "00"),
        (1, "01"),
        (10, "0a"),
        (23, "17"),
        (24, "18 18"),
        (25, "18 19"),
        (100, "18 64"),
        (1000, "19 03 e8"),
        (1_000_000, "1a 00 0f 42 40"),
        (1_000_000_000_000, "1b 00 00 00 e8 d4 a5 10 00"),
        (u64::MAX, "1b ff ff ff ff ff ff ff ff"),
    ];
    for (value, bytes) in cases {
        let expected = hex(bytes);
        assert_eq!(to_vec(value).unwrap(), expected, "encode {value}");
        assert_eq!(
            from_slice::<u64>(&expected).unwrap(),
            *value,
            "decode {value}"
        );
    }
}

#[test]
fn negative_integers() {
    let cases: &[(i64, &str)] = &[
        (-1, "20"),
        (-10, "29"),
        (-100, "38 63"),
        (-1000, "39 03 e7"),
        (i64::MIN, "3b 7f ff ff ff ff ff ff ff"),
    ];
    for (value, bytes) in cases {
        let expected = hex(bytes);
        assert_eq!(to_vec(value).unwrap(), expected, "encode {value}");
        assert_eq!(
            from_slice::<i64>(&expected).unwrap(),
            *value,
            "decode {value}"
        );
    }
}

#[test]
fn floats_use_smallest_lossless_width() {
    let cases: &[(f64, &str)] = &[
        (0.0, "f9 00 00"),
        (-0.0, "f9 80 00"),
        (1.0, "f9 3c 00"),
        (1.5, "f9 3e 00"),
        (-4.0, "f9 c4 00"),
        (65504.0, "f9 7b ff"),
        (0.00006103515625, "f9 04 00"),
        (5.960464477539063e-8, "f9 00 01"),
        (100000.0, "fa 47 c3 50 00"),
        (3.4028234663852886e38, "fa 7f 7f ff ff"),
        (1.1, "fb 3f f1 99 99 99 99 99 9a"),
        (f64::INFINITY, "f9 7c 00"),
        (f64::NEG_INFINITY, "f9 fc 00"),
        (f64::NAN, "f9 7e 00"),
    ];
    for (value, bytes) in cases {
        let expected = hex(bytes);
        assert_eq!(to_vec(value).unwrap(), expected, "encode {value:?}");
        let decoded: f64 = from_slice(&expected).unwrap();
        if value.is_nan() {
            assert!(decoded.is_nan());
        } else {
            assert_eq!(decoded.to_bits(), value.to_bits(), "decode {value:?}");
        }
    }
}

#[test]
fn f32_encodes_by_value() {
    assert_eq!(to_vec(&1.5f32).unwrap(), hex("f9 3e 00"));
    assert_eq!(to_vec(&0.1f32).unwrap(), hex("fa 3d cc cc cd"));
    assert_eq!(from_slice::<f32>(&hex("fa 3d cc cc cd")).unwrap(), 0.1f32);
}

#[test]
fn simple_values() {
    assert_eq!(to_vec(&false).unwrap(), hex("f4"));
    assert_eq!(to_vec(&true).unwrap(), hex("f5"));
    assert_eq!(to_vec(&Option::<u8>::None).unwrap(), hex("f6"));
    assert!(from_slice::<bool>(&hex("f5")).unwrap());
    assert_eq!(from_slice::<Option<u8>>(&hex("f6")).unwrap(), None);
    assert_eq!(from_slice::<Option<u8>>(&hex("07")).unwrap(), Some(7));
}

#[test]
fn strings_and_bytes() {
    assert_eq!(to_vec(&"").unwrap(), hex("60"));
    assert_eq!(to_vec(&"a").unwrap(), hex("61 61"));
    assert_eq!(to_vec(&"IETF").unwrap(), hex("64 49 45 54 46"));
    assert_eq!(to_vec(&"\u{fc}").unwrap(), hex("62 c3 bc"));
    assert_eq!(from_slice::<&str>(&hex("62 c3 bc")).unwrap(), "\u{fc}");

    let empty: &[u8] = &[];
    assert_eq!(to_vec(&empty).unwrap(), hex("40"));
    let four: &[u8] = &[1, 2, 3, 4];
    assert_eq!(to_vec(&four).unwrap(), hex("44 01 02 03 04"));
    assert_eq!(from_slice::<&[u8]>(&hex("44 01 02 03 04")).unwrap(), four);
}

#[test]
fn arrays_and_tuples() {
    assert_eq!(to_vec(&Vec::<u8>::new()).unwrap(), hex("80"));
    assert_eq!(to_vec(&vec![1u8, 2, 3]).unwrap(), hex("83 01 02 03"));
    let nested = (1u8, (2u8, 3u8), (4u8, 5u8));
    assert_eq!(to_vec(&nested).unwrap(), hex("83 01 82 02 03 82 04 05"));
    assert_eq!(from_slice::<(u8, u8)>(&hex("82 01 02")).unwrap(), (1, 2));
    assert_eq!(
        from_slice::<[u8; 3]>(&hex("83 01 02 03")).unwrap(),
        [1, 2, 3]
    );
    assert_eq!(
        from_slice::<[u8; 2]>(&hex("83 01 02 03")),
        Err(tpt_cbor::DecodeError::TypeMismatch)
    );
}

#[test]
fn value_arrays_and_maps() {
    let v = Value::Array(vec![
        Value::Integer(1),
        Value::Array(vec![Value::Integer(2), Value::Integer(3)]),
        Value::Array(vec![Value::Integer(4), Value::Integer(5)]),
    ]);
    assert_eq!(to_vec(&v).unwrap(), hex("83 01 82 02 03 82 04 05"));
    assert_eq!(
        from_slice::<Value<'_>>(&hex("83 01 82 02 03 82 04 05")).unwrap(),
        v
    );

    assert_eq!(to_vec(&Value::Map(vec![])).unwrap(), hex("a0"));
    let m = Value::Map(vec![
        (Value::Integer(1), Value::Integer(2)),
        (Value::Integer(3), Value::Integer(4)),
    ]);
    assert_eq!(to_vec(&m).unwrap(), hex("a2 01 02 03 04"));

    let m2 = Value::Map(vec![
        (Value::Text("a"), Value::Integer(1)),
        (
            Value::Text("b"),
            Value::Array(vec![Value::Integer(2), Value::Integer(3)]),
        ),
    ]);
    assert_eq!(to_vec(&m2).unwrap(), hex("a2 61 61 01 61 62 82 02 03"));
    assert_eq!(
        from_slice::<Value<'_>>(&hex("a2 61 61 01 61 62 82 02 03")).unwrap(),
        m2
    );

    let mixed = Value::Array(vec![
        Value::Text("a"),
        Value::Map(vec![(Value::Text("b"), Value::Text("c"))]),
    ]);
    assert_eq!(to_vec(&mixed).unwrap(), hex("82 61 61 a1 61 62 61 63"));
}

#[test]
fn value_scalars_round_trip() {
    let cases: Vec<(Value<'_>, &str)> = vec![
        (Value::Integer(-7), "26"),
        (Value::Integer(i64::MAX), "1b 7f ff ff ff ff ff ff ff"),
        (Value::Bytes(&[0xde, 0xad]), "42 de ad"),
        (Value::Text("hi"), "62 68 69"),
        (Value::Float(1.0), "f9 3c 00"),
        (Value::Bool(true), "f5"),
        (Value::Null, "f6"),
    ];
    for (value, bytes) in cases {
        let expected = hex(bytes);
        assert_eq!(to_vec(&value).unwrap(), expected);
        assert_eq!(from_slice::<Value<'_>>(&expected).unwrap(), value);
    }
}
