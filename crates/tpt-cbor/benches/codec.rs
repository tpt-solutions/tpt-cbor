//! Dependency-free micro-benchmarks. Run with `cargo bench -p tpt-cbor`.
//!
//! These are coarse throughput numbers only. They are not a regression gate.

use std::hint::black_box;
use std::time::{Duration, Instant};

use tpt_cbor::{from_slice, to_vec, Decoder, Value};

fn sample() -> Value<'static> {
    let items = (0..64)
        .map(|i| {
            Value::Map(vec![
                (Value::Text("id"), Value::Integer(i)),
                (Value::Text("name"), Value::Text("sensor-reading")),
                (Value::Text("value"), Value::Float(i as f64 * 0.25)),
                (
                    Value::Text("tags"),
                    Value::Array(vec![Value::Bool(true), Value::Null]),
                ),
            ])
        })
        .collect();
    Value::Array(items)
}

fn bench<F: FnMut()>(name: &str, bytes: usize, mut f: F) {
    // Warm up, then run for about half a second.
    for _ in 0..100 {
        f();
    }
    let budget = Duration::from_millis(500);
    let start = Instant::now();
    let mut iters: u64 = 0;
    while start.elapsed() < budget {
        f();
        iters += 1;
    }
    let elapsed = start.elapsed().as_secs_f64();
    let per_iter_ns = elapsed * 1e9 / iters as f64;
    let mib_s = (bytes as f64 * iters as f64) / elapsed / (1024.0 * 1024.0);
    println!("{name:<28} {per_iter_ns:>12.0} ns/iter {mib_s:>10.1} MiB/s");
}

fn main() {
    let value = sample();
    let encoded = to_vec(&value).expect("encode sample");
    println!("sample size: {} bytes", encoded.len());

    bench("encode (Value -> Vec)", encoded.len(), || {
        black_box(to_vec(black_box(&value)).expect("encode"));
    });
    bench("decode (full Value tree)", encoded.len(), || {
        let v: Value<'_> = from_slice(black_box(&encoded)).expect("decode");
        black_box(v);
    });
    bench("validate (skip_item, no alloc)", encoded.len(), || {
        let mut d = Decoder::new(black_box(&encoded));
        d.skip_item().expect("skip");
        black_box(&d);
    });
}
