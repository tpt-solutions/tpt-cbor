# No-std and embedded use

tpt-cbor is `#![no_std]`. It needs no allocator unless you enable the `alloc`
feature, which is on by default.

## Choosing features

```toml
[dependencies]
# Borrowed decoding and encoding into any Sink. No allocator needed.
tpt-cbor = { version = "0.1", default-features = false }
```

| Features | What you get |
|----------|--------------|
| none (`default-features = false`) | Encoder into any `Sink`; borrowed decoding (`&str`, `&[u8]`, integers, floats, `bool`, `Option`, tuples up to 8, `()`, `char`); `validate`, `is_canonical`, `sequence`; `#[derive]` for types whose fields need no allocation (with `derive`). |
| `alloc` (default) | Adds `Value`, `OwnedValue`, `ByteBuf`, `Vec`, `String`, `Box`, `BTreeMap`, `Cow` codecs, `to_vec`, `canonicalize`, the `cbor!` macro. |
| `std` | Adds `io` adapters (`to_writer`, `from_reader`), `HashMap` codecs, and `IoSink`. Implies `alloc`. |
| `derive` | `#[derive(Encode, Decode)]`. Build-time proc-macro only, so it adds no code to the target. |

Without `alloc`, these are not available: `Value`, `OwnedValue`, `ByteBuf`,
decoding into `Vec<T>`, `String`, `[T; N]`, `Box<T>`, `BTreeMap`, or `Cow`
(`Cow` encoding also needs `alloc`). Encoding `[T; N]` works without `alloc`,
but decoding it does not, because the decode buffer is a `Vec`.

## Encoding into a fixed buffer

Encoding writes through a `Sink`. Implement `Sink` for your buffer, and report
`EncodeError::BufferFull` when it is full:

```rust
use tpt_cbor::{Encode, EncodeError, Encoder, Sink};

struct Fixed<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Sink for Fixed<N> {
    fn write_all_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let end = self.len.checked_add(bytes.len()).ok_or(EncodeError::BufferFull)?;
        let dst = self.bytes.get_mut(self.len..end).ok_or(EncodeError::BufferFull)?;
        dst.copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}
```

The complete example, with a type that borrows its text on decode, is in
`crates/tpt-cbor/examples/no_std_embedded.rs`. Run it without default features:

```sh
cargo run -p tpt-cbor --example no_std_embedded --no-default-features
```

## Decoding without allocation

Decode from a buffer that outlives the value. Strings and byte strings borrow
from it, so nothing is copied:

```rust
use tpt_cbor::{from_slice, Config};

let bytes: &[u8] = &[0x65, b'l', b'a', b'b', b'e', b'l', 0x01];
// `&str` and `&[u8]` fields borrow from `bytes`.
let text: &str = from_slice(&bytes[..6]).unwrap();
assert_eq!(text, "label");
```

Use `tpt_cbor::validate(input, Config::strict())` to check untrusted input
without decoding it at all. It allocates nothing.

## Verifying the build

CI builds the core for `thumbv7em-none-eabihf`, both with and without default
features, and checks that the core crate has no dependencies. To reproduce
locally:

```sh
rustup target add thumbv7em-none-eabihf
cargo build -p tpt-cbor --no-default-features --target thumbv7em-none-eabihf
cargo build -p tpt-cbor --target thumbv7em-none-eabihf
```
