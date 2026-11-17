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
* **Transparent struct**: encodes exactly as its single field.

Decoding a map ignores unknown keys, which allows forward compatibility, unless the type has
`deny_unknown_fields`. Strict decoders still reject out-of-order or duplicate keys.

## Container attributes

| Attribute | Effect |
|-----------|--------|
| `#[cbor(rename_all = "...")]` | Renames the keys of a struct's named fields, or the names of an enum's variants. Rules: `lowercase`, `UPPERCASE`, `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case`, `SCREAMING-KEBAB-CASE`. A field's or variant's own `rename` takes precedence. The rule does not reach the fields of struct variants. |
| `#[cbor(deny_unknown_fields)]` | A map with an unknown key fails with `UnknownField`. Applies to named structs and struct variants. |
| `#[cbor(transparent)]` | On a struct with exactly one field. The struct encodes and decodes exactly as that field. |

## Field attributes

| Attribute | Where | Effect |
|-----------|-------|--------|
| `#[cbor(rename = "key")]` | named field or variant | Uses `key` instead of the Rust name on the wire. |
| `#[cbor(skip)]` | named field | Not encoded. Decoded as `Default::default()`. |
| `#[cbor(default)]` | named field | A missing key decodes to `Default::default()`. `Option<T>` fields behave this way automatically. |
| `#[cbor(skip_serializing_if = "path")]` | named field | The key is left out when `path(&field)` returns `true`, for example `"Option::is_none"`. |
| `#[cbor(with = "module")]` | named field | Encoded by `module::encode(&field, encoder)` and decoded by `module::decode(decoder)`. The field type needs no `Encode` or `Decode` impl. |
| `#[cbor(bytes)]` | `Vec<u8>` or `&[u8]` field | Encoded as a CBOR byte string rather than an array of integers. |

Attributes that do not apply to the item they sit on are rejected at compile time.

## Limitations

* At most one lifetime parameter is supported. It becomes the decode lifetime.
* Unions are rejected.

## Testing

```sh
cargo test -p tpt-cbor-derive-tests                # behaviour tests and compile-fail cases
cargo run -p tpt-cbor-derive-tests --example derive_config
```

Build-time dependencies are `syn`, `quote` and `proc-macro2`. None are linked into `tpt-cbor` itself.

## Minimum supported Rust version

Rust **1.85**.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Licensed under either of Apache License, Version 2.0 or MIT license, at your option.
See [LICENSE-APACHE](https://github.com/TPT-Solutions/tpt-cbor/blob/master/LICENSE-APACHE) and
[LICENSE-MIT](https://github.com/TPT-Solutions/tpt-cbor/blob/master/LICENSE-MIT).
