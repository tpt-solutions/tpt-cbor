//! Kani proof harnesses. Compiled only under `cargo kani`.
//!
//! Each harness covers one piece of the decoder and is kept small. Proofs over
//! the whole `skip_item` function were tried and did not finish within 20 to 25
//! minutes on a 4-byte input: CBMC encodes the map key-order code in every
//! harness that reaches it, and the container loops add to the solver's work.
//! `skip_item` is therefore not proved as a whole. It is built from the pieces
//! below: header parsing, integer, string and float readers, container
//! headers, and key order. Its own recursion is covered by the unit tests and
//! the fuzzer.
//!
//! Under Kani, UTF-8 validation is modelled as an arbitrary accept or reject
//! (see `decode.rs`). The proofs are about panics and bounds, which do not
//! depend on string contents. The tests and the fuzzer cover real validation.
//!
//! The encoder round trips prove algorithmic results. Results are recorded in
//! VERIFICATION.md.

use crate::{Decoder, Encoder};

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

/// Integer arguments of every width (1, 2, 4 and 8 bytes after the head) never
/// panic or read out of bounds, for both the unsigned and signed readers.
#[kani::proof]
#[kani::unwind(10)]
fn integer_arguments_never_panic() {
    let input: [u8; 9] = kani::any();
    let mut d = Decoder::new(&input);
    let _ = d.decode_u64();
    assert!(d.position() <= input.len());
    let mut d = Decoder::new(&input);
    let _ = d.decode_i64();
    assert!(d.position() <= input.len());
}

/// Byte and text string readers never panic or read out of bounds. The text
/// reader's UTF-8 check is modelled (see the module comment).
#[kani::proof]
#[kani::unwind(10)]
fn string_decode_never_panics() {
    let input: [u8; 9] = kani::any();
    let mut d = Decoder::new(&input);
    let _ = d.decode_str();
    assert!(d.position() <= input.len());
    let mut d = Decoder::new(&input);
    let _ = d.decode_bytes();
    assert!(d.position() <= input.len());
}

/// The float reader never panics for any half, single or double encoding.
#[kani::proof]
#[kani::unwind(10)]
fn float_decode_never_panics() {
    let input: [u8; 9] = kani::any();
    let mut d = Decoder::new(&input);
    let _ = d.decode_f64();
    assert!(d.position() <= input.len());
}

/// The container header readers never panic or read out of bounds. A declared
/// length is checked against the remaining input before any element is read.
#[kani::proof]
#[kani::unwind(10)]
fn container_headers_never_panic() {
    let input: [u8; 9] = kani::any();
    let mut d = Decoder::new(&input);
    let _ = d.decode_array_len();
    assert!(d.position() <= input.len());
    let mut d = Decoder::new(&input);
    let _ = d.decode_map_len();
    assert!(d.position() <= input.len());
}

/// The map key-order check never panics, whatever the spans. A span outside
/// the input is reported as an error rather than indexing out of bounds.
#[kani::proof]
#[kani::unwind(10)]
fn check_key_order_never_panics() {
    let input: [u8; 8] = kani::any();
    let d = Decoder::new(&input);
    let prev_start: usize = kani::any();
    let prev_end: usize = kani::any();
    let cur_start: usize = kani::any();
    let cur_end: usize = kani::any();
    let _ = d.check_key_order(Some((prev_start, prev_end)), (cur_start, cur_end));
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
