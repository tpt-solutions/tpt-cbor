//! Property-style round-trip tests without external dependencies.
//!
//! Random `Value` trees are generated from a deterministic xorshift PRNG.
//! Each one must encode, decode, and re-encode to identical bytes. The
//! zero-allocation skip path must also accept the same bytes.
#![cfg(feature = "alloc")]

use tpt_cbor::{from_slice, to_vec, Decoder, Value};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Builds a random tree. Text and bytes are drawn from `pool`, which owns the
/// data so that the `Value` can borrow it.
fn gen_value<'a>(rng: &mut Rng, depth: u32, pool: &'a [String]) -> Value<'a> {
    let choices = if depth >= 4 { 7 } else { 9 };
    match rng.below(choices) {
        0 => Value::Integer(rng.next() as i64),
        1 => Value::Integer(rng.below(64) as i64 - 32),
        2 => {
            let s = &pool[rng.below(pool.len() as u64) as usize];
            Value::Text(s.as_str())
        }
        3 => {
            let s = &pool[rng.below(pool.len() as u64) as usize];
            Value::Bytes(s.as_bytes())
        }
        4 => Value::Float(f64::from_bits(rng.next())),
        5 => Value::Bool(rng.below(2) == 0),
        6 => Value::Null,
        7 => {
            let n = rng.below(4) as usize;
            Value::Array((0..n).map(|_| gen_value(rng, depth + 1, pool)).collect())
        }
        _ => {
            let n = rng.below(4) as usize;
            // Keys are drawn from integers and text so that duplicates are
            // possible. Duplicates are removed below, because encoding rejects them.
            let mut pairs: Vec<(Value<'a>, Value<'a>)> = Vec::with_capacity(n);
            for _ in 0..n {
                let key = if rng.below(2) == 0 {
                    Value::Integer(rng.below(16) as i64)
                } else {
                    let s = &pool[rng.below(pool.len() as u64) as usize];
                    Value::Text(s.as_str())
                };
                if !pairs.iter().any(|(k, _)| *k == key) {
                    pairs.push((key, gen_value(rng, depth + 1, pool)));
                }
            }
            Value::Map(pairs)
        }
    }
}

#[test]
fn random_values_round_trip_to_identical_bytes() {
    let pool: Vec<String> = [
        "",
        "a",
        "key",
        "ünïcode",
        "a much longer string value for width tests "
            .repeat(3)
            .as_str(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for case in 0..if cfg!(miri) { 200 } else { 5_000 } {
        let value = gen_value(&mut rng, 0, &pool);
        let bytes = to_vec(&value).unwrap_or_else(|e| panic!("case {case}: encode failed: {e}"));

        // Full decode, then re-encode. The bytes must be identical.
        let decoded: Value<'_> =
            from_slice(&bytes).unwrap_or_else(|e| panic!("case {case}: decode failed: {e}"));
        assert_eq!(to_vec(&decoded).unwrap(), bytes, "case {case}: not stable");

        // Zero-allocation validation accepts exactly the same bytes.
        let mut d = Decoder::new(&bytes);
        d.skip_item()
            .unwrap_or_else(|e| panic!("case {case}: skip failed: {e}"));
        assert_eq!(d.finish(), Ok(()));
    }
}

#[test]
fn values_without_nan_round_trip_exactly() {
    let pool = vec![String::from("x")];
    let mut rng = Rng(0x0123_4567_89ab_cdef);
    let mut checked = 0;
    for _ in 0..if cfg!(miri) { 200 } else { 5_000 } {
        let value = gen_value(&mut rng, 0, &pool);
        if contains_nan(&value) {
            continue;
        }
        let bytes = to_vec(&value).unwrap();
        let decoded: Value<'_> = from_slice(&bytes).unwrap();
        assert!(values_equal(&decoded, &value), "mismatch for {value:?}");
        checked += 1;
    }
    assert!(
        checked > if cfg!(miri) { 20 } else { 1_000 },
        "generator produced too few NaN-free cases"
    );
}

fn contains_nan(v: &Value<'_>) -> bool {
    match v {
        Value::Float(f) => f.is_nan(),
        Value::Array(items) => items.iter().any(contains_nan),
        Value::Map(pairs) => pairs
            .iter()
            .any(|(k, v)| contains_nan(k) || contains_nan(v)),
        _ => false,
    }
}

/// Structural equality that compares floats bitwise and ignores map order.
/// Encoding sorts map keys, so the decoded order can differ from the input.
fn values_equal(a: &Value<'_>, b: &Value<'_>) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| values_equal(p, q))
        }
        (Value::Map(x), Value::Map(y)) => {
            x.len() == y.len()
                && x.iter().all(|(k, v)| {
                    y.iter()
                        .any(|(k2, v2)| values_equal(k, k2) && values_equal(v, v2))
                })
        }
        _ => a == b,
    }
}
