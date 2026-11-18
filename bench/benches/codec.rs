//! Criterion benchmarks for tpt-cbor.
//!
//! Three paths are measured on the same record: derived encode and decode
//! (typed), the dynamic `Value` path, and zero-allocation validation.
//! Numbers depend on the machine. Compare runs on one machine, and use them to
//! spot regressions rather than to compare with other crates.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use tpt_cbor::{from_slice, to_vec, validate, Config, Decode, Encode, Value};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Record {
    id: u64,
    name: String,
    values: Vec<u32>,
}

fn sample() -> Record {
    Record {
        id: 1_234_567,
        name: "sensor-7".to_string(),
        values: (0..32).map(|i| i * 17).collect(),
    }
}

fn bench_typed(c: &mut Criterion) {
    let record = sample();
    let bytes = to_vec(&record).expect("encode");
    let mut group = c.benchmark_group("typed");
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("encode", |b| {
        b.iter(|| to_vec(black_box(&record)).expect("encode"))
    });
    group.bench_function("decode", |b| {
        b.iter(|| from_slice::<Record>(black_box(&bytes)).expect("decode"))
    });
    group.finish();
}

fn bench_value(c: &mut Criterion) {
    let record = sample();
    let bytes = to_vec(&record).expect("encode");
    let value: Value<'_> = from_slice(&bytes).expect("decode");
    let mut group = c.benchmark_group("value");
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("decode", |b| {
        b.iter(|| from_slice::<Value<'_>>(black_box(&bytes)).expect("decode"))
    });
    group.bench_function("encode", |b| {
        b.iter(|| to_vec(black_box(&value)).expect("encode"))
    });
    group.finish();
}

fn bench_validate(c: &mut Criterion) {
    let bytes = to_vec(&sample()).expect("encode");
    let mut group = c.benchmark_group("validate");
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("strict", |b| {
        b.iter(|| validate(black_box(&bytes), Config::strict()).expect("valid"))
    });
    group.finish();
}

criterion_group!(benches, bench_typed, bench_value, bench_validate);
criterion_main!(benches);
