# tpt-cbor-derive

Derive macros for [`tpt-cbor`](https://crates.io/crates/tpt-cbor): `#[derive(Encode)]` and `#[derive(Decode)]`.

**Do not depend on this crate directly.** Enable the `derive` feature of `tpt-cbor` and use
`tpt_cbor::{Encode, Decode}`:

```toml
[dependencies]
tpt-cbor = { version = "0.1", features = ["derive"] }
```

```rust,ignore
use tpt_cbor::{Decode, Encode};

#[derive(Encode, Decode)]
struct Settings {
    name: String,
    #[cbor(rename = "max-retries")]
    max_retries: u8,
}
```

## Wire format

* **Named struct**: a canonical map from text keys to fields. Keys are the field names, or their
  `rename` values. Entries are emitted in RFC 8949 §4.2.1 bytewise key order, which is fixed at compile time.
* **Tuple struct**: an array of its fields, in declaration order.
* **Unit struct**: an empty array.
* **Enum** (externally tagged): a unit variant is its name as a text string. A tuple variant is
  `{"Name": [fields...]}`. A struct variant is `{"Name": {fields...}}`.

Decoding a map ignores unknown keys, which allows forward compatibility. Strict decoders still reject
out-of-order or duplicate keys.

## Attributes

| Attribute | Where | Effect |
|-----------|-------|--------|
| `#[cbor(rename = "key")]` | named field or variant | Uses `key` instead of the Rust name on the wire. |
| `#[cbor(skip)]` | named field | Not encoded. Decoded as `Default::default()`. |
| `#[cbor(default)]` | named field | A missing key decodes to `Default::default()`. `Option<T>` fields behave this way automatically. |

## Limitations

* At most one lifetime parameter is supported. It becomes the decode lifetime.
* Unions are rejected.

## Minimum supported Rust version

Rust **1.85**.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Licensed under either of Apache License, Version 2.0 or MIT license, at your option.
See [LICENSE-APACHE](https://github.com/TPT-Solutions/tpt-cbor/blob/master/LICENSE-APACHE) and
[LICENSE-MIT](https://github.com/TPT-Solutions/tpt-cbor/blob/master/LICENSE-MIT).
