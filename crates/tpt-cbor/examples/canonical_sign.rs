//! Canonical bytes for signing.
//!
//! Two maps with the same entries, built in different orders, must produce
//! the same bytes. Only then can a digest of the encoding be checked by a
//! verifier that builds the map in its own order.
//!
//! Run with: `cargo run -p tpt-cbor --example canonical_sign`

use std::fmt::Write as _;

use tpt_cbor::{to_vec, Value};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn main() {
    // The sender builds the message with the amount first.
    let sender = Value::Map(vec![
        (Value::Text("amount"), Value::Integer(42)),
        (Value::Text("currency"), Value::Text("NZD")),
        (Value::Text("id"), Value::Text("inv-0001")),
    ]);

    // The verifier builds the same message in a different order.
    let verifier = Value::Map(vec![
        (Value::Text("id"), Value::Text("inv-0001")),
        (Value::Text("currency"), Value::Text("NZD")),
        (Value::Text("amount"), Value::Integer(42)),
    ]);

    let signed_bytes = to_vec(&sender).expect("encode sender message");
    let checked_bytes = to_vec(&verifier).expect("encode verifier message");
    assert_eq!(
        signed_bytes, checked_bytes,
        "canonical encoding must not depend on insertion order"
    );

    println!(
        "canonical message ({} bytes): {}",
        signed_bytes.len(),
        hex(&signed_bytes)
    );
    println!("both orders produce identical bytes");
    // A real system would now hash `signed_bytes` and sign the digest. That
    // step is outside this crate.
}
