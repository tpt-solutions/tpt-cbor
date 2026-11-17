# tpt-cbor

A pure-Rust, **zero-dependency** implementation of [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949).

* **Strict canonical encoding.** Shortest integers, shortest lossless floats (NaN is always `0xf97e00`),
  definite lengths only, and map keys in RFC 8949 §4.2.1 bytewise order. Identical data always produces identical bytes.
* **Strict decoding by default.** Non-canonical input is rejected. A lenient mode is available as an opt-in.
  Indefinite lengths, tags, `undefined`, and unassigned simple values are always rejected.
* **Zero-copy decoding.** Text and byte strings borrow from the input. Validation can run without allocating.
* **Panic-free and bounded.** No `unsafe` code. Nesting depth is capped (default 64). Arithmetic is checked.
* **No ecosystem lock-in.** Works with its own `Encode` and `Decode` traits. It does not depend on serde.

## Quick start

```rust
use tpt_cbor::{from_slice, to_vec, Value};

let value = Value::Map(vec![
    (Value::Text("a"), Value::Integer(1)),
    (Value::Text("b"), Value::Array(vec![Value::Integer(2), Value::Integer(3)])),
]);
let bytes = to_vec(&value)?;
assert_eq!(bytes, [0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0x82, 0x02, 0x03]);

let decoded: Value<'_> = from_slice(&bytes)?;
assert_eq!(decoded, value);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Feature flags

| Feature  | Default | Effect |
|----------|---------|--------|
| `alloc`  | yes     | `Value`, `Vec`/`String` codecs, `to_vec`. |
| `std`    | no      | `std::io` adapters: `to_writer`, `read_to_buffer`. Implies `alloc`. |
| `derive` | no      | `#[derive(Encode, Decode)]`. Pulls in `tpt-cbor-derive` and its build-time `syn`/`quote` dependencies. |

With `default-features = false` the crate is `#![no_std]` and needs no allocator.
Only borrowed decoding is available in that mode.

## Derive macros

With the `derive` feature, structs and enums can be encoded directly:

```rust,ignore
use tpt_cbor::{Decode, Encode};

#[derive(Encode, Decode)]
struct Settings {
    name: String,
    #[cbor(rename = "max-retries")]
    max_retries: u8,
}
```

Named structs become canonical maps keyed by field name. Enums are externally
tagged: a unit variant is its name as text, and other variants are
`{"Name": payload}`. See the [`tpt-cbor-derive` README](https://github.com/TPT-Solutions/tpt-cbor/blob/master/crates/tpt-cbor-derive/README.md)
for the full format and the `rename`, `skip` and `default` attributes.

## Verification

Proof harnesses for the integer, float and skip paths are written but have **not been run yet**.
Kani is not installed in the development environment. See
[`VERIFICATION.md`](https://github.com/TPT-Solutions/tpt-cbor/blob/master/VERIFICATION.md) for status.

## Minimum supported Rust version

Rust **1.85**.

## Testing

```sh
cargo test -p tpt-cbor                       # default features
cargo test -p tpt-cbor --features std        # std I/O adapters
cargo test -p tpt-cbor --no-default-features # no_std, no alloc
cargo bench -p tpt-cbor                      # coarse throughput numbers
cargo run -p tpt-cbor --example value_roundtrip
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/TPT-Solutions/tpt-cbor/blob/master/LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](https://github.com/TPT-Solutions/tpt-cbor/blob/master/LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work
by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms
or conditions.
