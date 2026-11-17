#![cfg(feature = "alloc")]
//! Malformed-input corpus, exhaustive header coverage, and a bounded
//! no-panic sweep. Every entry point is exercised on every input, and none
//! may panic.

use tpt_cbor::{from_slice, from_slice_with_config, Config, DecodeError, Decoder, Kind, Value};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

/// Runs every public decode entry point over `input`. Used by the exhaustive
/// tests, which only require that nothing panics.
fn probe_all(input: &[u8]) {
    let _ = from_slice::<Value<'_>>(input);
    let _ = from_slice_with_config::<Value<'_>>(input, Config::lenient());
    let _ = from_slice::<u64>(input);
    let _ = from_slice::<i64>(input);
    let _ = from_slice::<f64>(input);
    let _ = from_slice::<&str>(input);
    let _ = from_slice::<&[u8]>(input);
    let _ = from_slice::<Vec<Value<'_>>>(input);
    let _ = from_slice::<Option<Value<'_>>>(input);
    let mut d = Decoder::new(input);
    let _ = d.skip_item();
    let mut d = Decoder::with_config(input, Config::lenient());
    let _ = d.skip_item();
    let mut d = Decoder::new(input);
    let _ = d.peek_kind();
    let _ = d.decode_array_len();
    let mut d = Decoder::new(input);
    let _ = d.decode_map_len();
}

#[test]
fn truncated_input_is_eof() {
    assert_eq!(
        from_slice::<Value<'_>>(&hex("19 01")),
        Err(DecodeError::UnexpectedEof)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("1b 00 00")),
        Err(DecodeError::UnexpectedEof)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("82 01")),
        Err(DecodeError::UnexpectedEof)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("a1 01")),
        Err(DecodeError::UnexpectedEof)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("fb 3f f1")),
        Err(DecodeError::UnexpectedEof)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&[]),
        Err(DecodeError::UnexpectedEof)
    );
}

#[test]
fn declared_length_larger_than_input_is_eof_without_allocating() {
    // Byte string claiming 2^32 - 1 bytes, with none present.
    assert_eq!(
        from_slice::<&[u8]>(&hex("5a ff ff ff ff")),
        Err(DecodeError::UnexpectedEof)
    );
    // Array claiming 2^32 - 1 elements.
    assert_eq!(
        from_slice::<Value<'_>>(&hex("9a ff ff ff ff")),
        Err(DecodeError::UnexpectedEof)
    );
    // Map claiming 2^32 - 1 pairs.
    assert_eq!(
        from_slice::<Value<'_>>(&hex("ba ff ff ff ff")),
        Err(DecodeError::UnexpectedEof)
    );
    // 2^64 - 1 length, which cannot fit in usize on a 32-bit target either.
    assert_eq!(
        from_slice::<&[u8]>(&hex("5b ff ff ff ff ff ff ff ff")),
        Err(DecodeError::UnexpectedEof)
    );
}

#[test]
fn bad_utf8_is_rejected() {
    assert_eq!(
        from_slice::<&str>(&hex("61 ff")),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(
        from_slice::<&str>(&hex("62 c3 28")),
        Err(DecodeError::InvalidUtf8)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("62 ed a0 80")),
        Err(DecodeError::InvalidUtf8)
    );
}

#[test]
fn indefinite_lengths_are_always_rejected() {
    for input in ["5f ff", "7f ff", "9f ff", "bf ff"] {
        assert_eq!(
            from_slice_with_config::<Value<'_>>(&hex(input), Config::lenient()),
            Err(DecodeError::IndefiniteLength),
            "lenient {input}"
        );
        assert_eq!(
            from_slice::<Value<'_>>(&hex(input)),
            Err(DecodeError::IndefiniteLength),
            "strict {input}"
        );
    }
    // Indefinite text used through the typed path.
    assert_eq!(
        from_slice::<&str>(&hex("7f 61 61 ff")),
        Err(DecodeError::IndefiniteLength)
    );
}

#[test]
fn stray_break_and_reserved_info_are_malformed() {
    assert_eq!(
        from_slice::<Value<'_>>(&hex("ff")),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("1c")),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("3d")),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("fc")),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("df")),
        Err(DecodeError::Malformed)
    );
}

#[test]
fn tags_undefined_and_unassigned_simple_values_are_unsupported() {
    assert_eq!(
        from_slice::<Value<'_>>(&hex("c0 00")),
        Err(DecodeError::UnsupportedType)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("d8 20 00")),
        Err(DecodeError::UnsupportedType)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("f7")),
        Err(DecodeError::UnsupportedType)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("f0")),
        Err(DecodeError::UnsupportedType)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("f8 20")),
        Err(DecodeError::UnsupportedType)
    );
    assert_eq!(
        from_slice::<Value<'_>>(&hex("e0")),
        Err(DecodeError::UnsupportedType)
    );
}

#[test]
fn integer_overflow_is_reported() {
    // 2^63 does not fit in i64.
    assert_eq!(
        from_slice::<i64>(&hex("1b 80 00 00 00 00 00 00 00")),
        Err(DecodeError::Overflow)
    );
    // -2^63 - 1 does not fit in i64.
    assert_eq!(
        from_slice::<i64>(&hex("3b 80 00 00 00 00 00 00 00")),
        Err(DecodeError::Overflow)
    );
    // u64::MAX as Value: outside Value::Integer's range.
    assert_eq!(
        from_slice::<Value<'_>>(&hex("1b ff ff ff ff ff ff ff ff")),
        Err(DecodeError::Overflow)
    );
    // Narrowing: 256 does not fit in u8.
    assert_eq!(
        from_slice::<u8>(&hex("19 01 00")),
        Err(DecodeError::Overflow)
    );
    // -129 does not fit in i8.
    assert_eq!(from_slice::<i8>(&hex("38 80")), Err(DecodeError::Overflow));
}

#[test]
fn trailing_bytes_are_rejected() {
    assert_eq!(
        from_slice::<u8>(&hex("01 02")),
        Err(DecodeError::TrailingBytes)
    );
}

#[test]
fn type_mismatch_is_reported() {
    assert_eq!(
        from_slice::<u8>(&hex("61 61")),
        Err(DecodeError::TypeMismatch)
    );
    assert_eq!(
        from_slice::<&str>(&hex("01")),
        Err(DecodeError::TypeMismatch)
    );
    assert_eq!(
        from_slice::<bool>(&hex("00")),
        Err(DecodeError::TypeMismatch)
    );
}

#[test]
fn depth_limit_is_enforced() {
    let nested = |levels: usize| {
        let mut bytes = vec![0x81u8; levels];
        bytes.push(0x00);
        bytes
    };
    // Exactly the default limit (64) is accepted.
    assert!(from_slice::<Value<'_>>(&nested(64)).is_ok());
    // One past the limit is rejected.
    assert_eq!(
        from_slice::<Value<'_>>(&nested(65)),
        Err(DecodeError::DepthExceeded)
    );
    // The zero-allocation skip path enforces the same limit.
    let too_deep = nested(65);
    let mut d = Decoder::new(&too_deep);
    assert_eq!(d.skip_item(), Err(DecodeError::DepthExceeded));
    // A pathological input does not overflow the stack.
    let deep = nested(100_000);
    assert_eq!(
        from_slice::<Value<'_>>(&deep),
        Err(DecodeError::DepthExceeded)
    );
    // Raising the limit allows more depth, up to the hard ceiling.
    let cfg = Config {
        strict: true,
        max_depth: 1_000_000,
    };
    assert!(from_slice_with_config::<Value<'_>>(&nested(256), cfg).is_ok());
    assert_eq!(
        from_slice_with_config::<Value<'_>>(&nested(257), cfg),
        Err(DecodeError::DepthExceeded)
    );
}

#[test]
fn peek_kind_classifies_without_consuming() {
    let first = hex("18 18");
    let d = Decoder::new(&first);
    assert_eq!(d.peek_kind(), Ok(Kind::Unsigned));
    assert_eq!(d.position(), 0);
    let cases: [(&str, Result<Kind, DecodeError>); 8] = [
        ("20", Ok(Kind::Negative)),
        ("41 00", Ok(Kind::Bytes)),
        ("61 00", Ok(Kind::Text)),
        ("80", Ok(Kind::Array)),
        ("a0", Ok(Kind::Map)),
        ("f5", Ok(Kind::Bool)),
        ("f6", Ok(Kind::Null)),
        ("f9 00 00", Ok(Kind::Float)),
    ];
    for (input, expected) in cases {
        let bytes = hex(input);
        assert_eq!(Decoder::new(&bytes).peek_kind(), expected, "{input}");
    }
    assert_eq!(
        Decoder::new(&[]).peek_kind(),
        Err(DecodeError::UnexpectedEof)
    );
}

/// Every first byte, with every possible second byte, through every entry point.
/// Covers all header shapes: each major type with each additional-info value.
#[test]
fn exhaustive_first_two_bytes_never_panic() {
    for first in 0u16..=255 {
        probe_all(&[first as u8]);
        for second in 0u16..=255 {
            probe_all(&[first as u8, second as u8]);
        }
    }
}

/// Every first byte with each 8-byte argument shape that is a 9-byte head.
/// Also covers the 3-byte and 5-byte argument widths with edge values.
#[test]
fn exhaustive_argument_widths_never_panic() {
    let args: [&[u8]; 6] = [
        &[0x00, 0x00],
        &[0xff, 0xff],
        &[0x80, 0x00, 0x00, 0x00],
        &[0xff, 0xff, 0xff, 0xff],
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    ];
    for first in 0u16..=255 {
        for arg in args {
            let mut input = vec![first as u8];
            input.extend_from_slice(arg);
            probe_all(&input);
        }
    }
}

/// Deterministic xorshift sweep of random short inputs. Not a substitute for
/// fuzzing (see fuzz/), but it is a cheap regression guard for panics.
#[test]
fn random_inputs_never_panic() {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..50_000 {
        let len = (next() % 40) as usize;
        let input: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        probe_all(&input);
    }
}

#[test]
fn decoder_position_and_finish() {
    let bytes = hex("82 01 02");
    let mut d = Decoder::new(&bytes);
    assert_eq!(d.decode_array_len(), Ok(2));
    assert_eq!(d.decode_u64(), Ok(1));
    assert_eq!(d.remaining(), 1);
    assert_eq!(d.finish(), Err(DecodeError::TrailingBytes));
    assert_eq!(d.decode_u64(), Ok(2));
    assert_eq!(d.position(), 3);
    assert_eq!(d.finish(), Ok(()));
}
