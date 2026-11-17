# Cookbook

Short recipes. Each one names the example or test where the full code lives.

## Build a dynamic value

```rust
use tpt_cbor::{cbor, to_vec};

let message = cbor!({ "id": "inv-0001", "amount": 42, "tags": ["a", "b"] });
let bytes = to_vec(&message).unwrap();
println!("{message}");   // {"id": "inv-0001", "amount": 42, "tags": ["a", "b"]}
```

The `cbor!` macro takes literals, arrays and maps. Write a negative number in
parentheses, as `(-1)`. Use `Value::from` for anything more complex.

## Get the same bytes however the map was built

Maps are sorted by their encoded keys (RFC 8949 section 4.2.1), so insertion
order does not matter. See `examples/canonical_sign.rs`.

## Decode untrusted input safely

Set explicit limits, and report errors with their byte offset. See
`examples/hostile_input.rs`.

```rust
use tpt_cbor::{from_slice_located_with_config, Config, Value};

let limits = Config { max_depth: 16, max_items: 1_000, ..Config::strict() };
match from_slice_located_with_config::<Value<'_>>(input, limits) {
    Ok(value) => { /* use value */ }
    Err(located) => eprintln!("rejected: {located}"),   // includes the byte offset
}
```

## Accept non-canonical input, but store canonical bytes

Strict mode rejects non-canonical input. Lenient mode accepts it, and
`canonicalize` rewrites the result:

```rust
let canonical: Vec<u8> = tpt_cbor::canonicalize(input)?;
```

## Read a stream of items (RFC 8742)

`sequence` yields one item per call, until the input is used up. After an
error it yields that error once, then stops:

```rust
for item in tpt_cbor::sequence::<Value<'_>>(&bytes) {
    let value = item?;
    // process value
}
```

## Owned values that outlive the input

`Value<'a>` borrows from the input. Use `OwnedValue` to keep data after the
buffer is gone, or convert with `into_owned`:

```rust
use tpt_cbor::{from_slice, OwnedValue};

let owned: OwnedValue = from_slice(&bytes)?;
```

## Binary data

`Vec<u8>` is an array of integers, not a byte string. Use one of:

* `ByteBuf`: an owned byte string.
* `&[u8]`: a borrowed byte string.
* `#[cbor(bytes)]` on a `Vec<u8>` or `&[u8]` struct field.

## Derived types

```rust
use tpt_cbor::{Decode, Encode};

#[derive(Encode, Decode)]
#[cbor(rename_all = "camelCase", deny_unknown_fields)]
struct Account {
    account_id: u64,
    #[cbor(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
    #[cbor(bytes)]
    key_material: Vec<u8>,
}
```

This encodes with the keys `accountId`, `keyMaterial` and, when present,
`nickname`. Decoding rejects any other key. See the `tpt-cbor-derive` crate
documentation for every attribute.

## Encode a field with your own codec

Use `with` when a field needs a custom encoding and its type has no `Encode`
impl:

```rust
mod hex_u32 {
    use tpt_cbor::{DecodeError, Decoder, EncodeError, Encoder};

    pub fn encode(v: &u32, e: &mut Encoder<'_>) -> Result<(), EncodeError> {
        e.encode_str(&format!("{v:x}"))
    }

    pub fn decode(d: &mut Decoder<'_>) -> Result<u32, DecodeError> {
        u32::from_str_radix(d.decode_str()?, 16).map_err(|_| DecodeError::Malformed)
    }
}
```

## Convert between CBOR and JSON, or inspect a file

Use the command-line tool. See [its README](../crates/tpt-cbor-cli/README.md):

```sh
tpt-cbor cbor-to-json -i message.cbor
tpt-cbor diag -i message.cbor
```
