# Changelog

All notable changes to `tpt-cbor` are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Strict canonical encoder and strict decoder, with an opt-in lenient mode and zero-copy borrowing.
- `Encode` / `Decode` traits with impls for integers, floats, bool, text, byte strings, `Vec`, `Option`, arrays and tuples.
- `Value` dynamic type (feature `alloc`), with canonical map ordering and duplicate-key rejection on encode.
- `std` feature: `to_writer`, `IoSink`, `read_to_buffer`.
- `derive` feature: re-exports the `Encode` and `Decode` derive macros from `tpt-cbor-derive`.
- `Config::max_items` (default `DEFAULT_MAX_ITEMS`, 2^20) and `DecodeError::LimitExceeded`. The limit applies to each array or map, and is checked after the input-length check.
- `Debug` for `Decoder` (omits input bytes) and `Encoder`.
- `From` impls, `Index<&str>` / `Index<usize>` (never panic), `Display` in diagnostic notation, and the `cbor!` macro for `Value`.
- `OwnedValue` and `Value::into_owned`; `ByteBuf` for owned byte strings.
- Codecs for `usize`, `isize`, `char`, `()`, `Box`, `Cow`, `BTreeMap`, `HashMap` (feature `std`), `[u8]`, and tuples up to arity 8.
- `Located`, with `from_slice_located` and `from_slice_located_with_config`, for errors with a byte offset.
- `validate`, `is_canonical`, `canonicalize`, `sequence` and `sequence_with_config` (RFC 8742), `from_reader` (feature `std`), and `prelude`.
- `EncodeError::BufferFull` for fixed-capacity sinks, and `DecodeError::UnknownField`.

### Changed

- `check_key_order` reports `DecodeError::Malformed` for a span outside the input, not `UnexpectedEof`.
- Containers reserve at most 1024 slots before decoding elements (`PREALLOC_LIMIT`).
- Decoding `Value` arrays and maps no longer reserves capacity from the declared length alone.
- The workspace uses resolver 3, which the 1.85 MSRV supports.

### Verification

- Kani proofs cover header parsing, integer and float encoding, the key-order check, and container depth (`container_depth_never_exceeds_limit`). Proofs are test-only and do not change the public API.

### Fixed

- The README quick start is compiled and run as a doctest (feature `alloc`).
- `value_roundtrip` example requires `alloc`, so `--no-default-features` builds pass.

### Notes

- Map keys use RFC 8949 §4.2.1 bytewise order.
- Tags, `undefined`, and unassigned simple values are rejected.
- Zero runtime dependencies (the `derive` feature adds build-time proc-macro dependencies only).
