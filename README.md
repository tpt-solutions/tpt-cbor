# tpt-cbor

A pure-Rust, **zero-dependency** implementation of [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949).
It is the binary serialization layer of the TPT pure-Rust stack.

* **Strict canonical encoding.** Shortest integers, shortest lossless floats (NaN is always `0xf97e00`),
  definite lengths only, and map keys in RFC 8949 §4.2.1 bytewise order. Identical data always produces identical bytes.
* **Strict decoding by default.** Non-canonical input is rejected. A lenient mode is available as an opt-in.
  Indefinite lengths, tags, `undefined`, and unassigned simple values are always rejected.
* **Zero-copy decoding.** Text and byte strings borrow from the input. Validation can run without allocating.
* **Panic-free and bounded.** No `unsafe` code. Nesting depth is capped (default 64), each container's length is
  capped (default 2^20 items), and arithmetic is checked.
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

The `cbor!` macro builds the same value from JSON-like syntax:

```rust
use tpt_cbor::{cbor, to_vec};

let value = cbor!({ "a": 1, "b": [2, 3] });
assert_eq!(
    to_vec(&value)?,
    [0xa2, 0x61, 0x61, 0x01, 0x61, 0x62, 0x82, 0x02, 0x03]
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Decoding

Strict decoding is the default. `from_slice` accepts exactly one data item and rejects trailing bytes.

```rust
use tpt_cbor::{from_slice, from_slice_located, from_slice_with_config, Config};

// Non-minimal encodings are rejected in strict mode.
assert!(from_slice::<u64>(&[0x18, 0x01]).is_err());

// Lenient mode accepts them.
assert_eq!(from_slice_with_config::<u64>(&[0x18, 0x01], Config::lenient()), Ok(1));

// On failure, `from_slice_located` also reports the byte offset where decoding stopped
// (here, after the header byte, when the 2-byte argument is cut short).
let err = from_slice_located::<u64>(&[0x19, 0x03]).unwrap_err();
assert_eq!(err.offset, 1);
```

Decoding into an owned type works without a lifetime. `ByteBuf` is an owned byte string, and
`OwnedValue` is the owned counterpart of `Value`.

Whole-input helpers work on raw bytes:

| Function | Purpose |
|----------|---------|
| `validate(input, config)` | Checks that the input is exactly one well-formed item. Allocates nothing. |
| `is_canonical(input)` | `true` if the input is exactly one canonical item. |
| `canonicalize(input)` | Re-encodes the input in canonical form. Requires `alloc`. |

For RFC 8742 CBOR sequences (items laid end to end), use `sequence(input)`, which returns an
iterator of `Result<T, DecodeError>`. Iteration stops after the first error.

## Feature flags

| Feature | Default | Effect |
|---------|---------|--------|
| `alloc` | yes     | `Value`, `OwnedValue`, `ByteBuf`, `Vec`/`String` codecs, `to_vec`, `canonicalize`, `cbor!`. |
| `std`   | no      | `std::io` adapters: `to_writer`, `from_reader`, `read_to_buffer`, `IoSink`. Implies `alloc`. |
| `derive` | no     | `#[derive(Encode, Decode)]`. Pulls in `tpt-cbor-derive` and its build-time `syn`/`quote` dependencies. |

With `default-features = false` the crate is `#![no_std]` and needs no allocator.
Only borrowed decoding is available in that mode.

`use tpt_cbor::prelude::*;` imports the commonly used items (`Encode`, `Decode`, `Encoder`, `Decoder`,
`from_slice`, and so on). With `alloc`, it also imports `to_vec`, `cbor!`, `Value` and `OwnedValue`.

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
`{"Name": payload}`. See the `tpt-cbor-derive` crate docs for the full format and
the container and field attributes (`rename`, `rename_all`, `skip`, `default`, `skip_serializing_if`, `with`, `bytes`, `transparent`, `deny_unknown_fields`).

## Verification

Proof harnesses for the integer, float and skip paths are written
(`crates/tpt-cbor/src/proofs.rs`) but have **not been run yet**. Kani is not
installed here. See [`VERIFICATION.md`](VERIFICATION.md) for status, including
the RFC 8949 Appendix A vectors, the fuzz harness and the other checks.

## Minimum supported Rust version

Rust **1.85**. The toolchain is pinned in `rust-toolchain.toml`.

## Workspace layout

| Crate | Purpose |
|-------|---------|
| `crates/tpt-cbor` | The core codec. Zero dependencies. |
| `crates/tpt-cbor-derive` | `Encode` and `Decode` derive macros (opt-in `derive` feature). |
| `fuzz/` | `cargo-fuzz` harness. Outside the workspace and never published. |

Design notes are in [`spec.txt`](spec.txt). Work tracking is in [`todo.md`](todo.md).
Release history is in [`CHANGELOG.md`](CHANGELOG.md).

## Testing

```sh
cargo test --workspace --all-features           # all crates, all features
cargo test -p tpt-cbor                          # default features
cargo test -p tpt-cbor --features std           # std I/O adapters
cargo test -p tpt-cbor --no-default-features    # no_std, no alloc
cargo test -p tpt-cbor-derive-tests --test compile_fail  # derive compile-fail cases
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo fmt --all -- --check
cargo bench -p tpt-cbor                         # coarse throughput numbers
cargo run -p tpt-cbor --example value_roundtrip
cargo run -p tpt-cbor-derive-tests --example derive_config
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
