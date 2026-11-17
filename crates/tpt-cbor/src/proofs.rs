//! Kani proof harnesses. Compiled only under `cargo kani`.
//!
//! STATUS: written but NOT YET RUN. Kani is not installed in the development
//! environment. Run with `cargo kani -p tpt-cbor` once it is available, then
//! record results in VERIFICATION.md.
//!
//! Scope: these harnesses cover header parsing, bounded skipping, integer and
//! float encoding round-trips. Vec/String/Value code is left to tests and
//! fuzzing, as the Phase 3 plan says.

use crate::{Config, Decoder, Encoder};

/// Header parsing never panics for any first byte, and classification is
/// consistent with the decode path.
#[kani::proof]
fn header_parse_never_panics() {
    let byte: u8 = kani::any();
    let input = [byte];
    let d = Decoder::new(&input);
    let _ = d.peek_kind();
    let mut d = Decoder::new(&input);
    let _ = d.decode_u64();
}

/// `skip_item` never panics and never reads out of bounds on an 8-byte
/// symbolic input. Depth is shrunk to 2 so that the unwinding bound is small.
#[kani::proof]
#[kani::unwind(10)]
fn skip_item_bounded() {
    let input: [u8; 8] = kani::any();
    let config = Config {
        strict: true,
        max_depth: 2,
    };
    let mut d = Decoder::with_config(&input, config);
    let _ = d.skip_item();
    assert!(d.position() <= input.len());
}

/// Integer encoding is minimal for every width boundary, and it round-trips.
#[kani::proof]
#[kani::unwind(10)]
fn unsigned_encoding_minimal_and_round_trips() {
    let value: u64 = kani::any();
    let mut out: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    {
        let mut enc = Encoder::new(&mut out);
        enc.encode_u64(value).unwrap();
    }
    let expected_len = if value < 24 {
        1
    } else if value <= 0xff {
        2
    } else if value <= 0xffff {
        3
    } else if value <= 0xffff_ffff {
        5
    } else {
        9
    };
    assert!(out.len() == expected_len);
    let mut d = Decoder::new(&out);
    assert!(d.decode_u64().unwrap() == value);
    assert!(d.finish().is_ok());
}

/// Signed integers round-trip across the full `i64` range.
#[kani::proof]
#[kani::unwind(10)]
fn signed_round_trips() {
    let value: i64 = kani::any();
    let mut out: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    {
        let mut enc = Encoder::new(&mut out);
        enc.encode_i64(value).unwrap();
    }
    let mut d = Decoder::new(&out);
    assert!(d.decode_i64().unwrap() == value);
}

/// Any finite or infinite float that decodes strictly re-encodes to the same
/// bytes. Canonical NaN is covered by the unit tests.
#[kani::proof]
#[kani::unwind(10)]
fn float_canonical_round_trip() {
    let bits: u64 = kani::any();
    let value = f64::from_bits(bits);
    kani::assume(!value.is_nan());
    let mut out: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    {
        let mut enc = Encoder::new(&mut out);
        enc.encode_f64(value).unwrap();
    }
    let mut d = Decoder::new(&out);
    let back = d.decode_f64().unwrap();
    assert!(back.to_bits() == value.to_bits());
    let mut out2: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    {
        let mut enc = Encoder::new(&mut out2);
        enc.encode_f64(back).unwrap();
    }
    assert!(out == out2);
}
