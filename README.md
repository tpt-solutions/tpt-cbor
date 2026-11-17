# tpt-cbor

A pure-Rust, **zero-dependency** implementation of [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949).
It is the binary serialization layer of the TPT pure-Rust stack.

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

| Feature | Default | Effect |
|---------|---------|--------|
| `alloc` | yes     | `Value`, `Vec`/`String` codecs, `to_vec`. |
| `std`   | no      | `std::io` adapters: `to_writer`, `read_to_buffer`. Implies `alloc`. |

With `default-features = false` the crate is `#![no_std]` and needs no allocator.
Only borrowed decoding is available in that mode.

## Minimum supported Rust version

Rust **1.85**. The toolchain is pinned in `rust-toolchain.toml`.

## Workspace layout

| Crate | Purpose |
|-------|---------|
| `crates/tpt-cbor` | The core codec. Zero dependencies. |
| `crates/tpt-cbor-derive` | Derive macros. Placeholder until Phase 2 of `todo.md`. |
| `fuzz/` | `cargo-fuzz` harness. Outside the workspace and never published. |

Design notes are in [`spec.txt`](spec.txt). Work tracking is in [`todo.md`](todo.md).

## Testing

```sh
cargo test --workspace                          # default features
cargo test -p tpt-cbor --features std           # std I/O adapters
cargo test -p tpt-cbor --no-default-features    # no_std, no alloc
cargo bench -p tpt-cbor                         # coarse throughput numbers
```

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work
by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms
or conditions.

Copyright (c) 2026 TPT Solutions.
