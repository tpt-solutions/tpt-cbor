//! Encode a dynamic `Value` canonically, then decode it back without copying.
//!
//! Run with `cargo run -p tpt-cbor --example value_roundtrip`.

use tpt_cbor::{from_slice, to_vec, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Keys are given in any order. The encoder sorts them into canonical order.
    let value = Value::Map(vec![
        (Value::Text("status"), Value::Text("ok")),
        (Value::Text("count"), Value::Integer(3)),
        (Value::Text("ratio"), Value::Float(0.5)),
    ]);

    let bytes = to_vec(&value)?;
    println!("canonical bytes ({} total): {bytes:02x?}", bytes.len());

    // Text borrows from `bytes`, so the decoded value must not outlive them.
    let decoded: Value<'_> = from_slice(&bytes)?;
    println!(
        "count = {:?}",
        decoded.get_text("count").and_then(Value::as_i64)
    );
    println!(
        "status = {:?}",
        decoded.get_text("status").and_then(Value::as_str)
    );
    // Decoded keys are in canonical order, which may differ from the input order.
    // Re-encoding must reproduce the exact bytes.
    assert_eq!(to_vec(&decoded)?, bytes);
    Ok(())
}
