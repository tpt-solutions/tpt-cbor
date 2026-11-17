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

### Changed

- `check_key_order` reports `DecodeError::Malformed` for a span outside the input, not `UnexpectedEof`.
- Containers reserve at most 1024 slots before decoding elements (`PREALLOC_LIMIT`).
- Decoding `Value` arrays and maps no longer reserves capacity from the declared length alone.

### Fixed

- The README quick start is compiled and run as a doctest (feature `alloc`).
- `value_roundtrip` example requires `alloc`, so `--no-default-features` builds pass.

### Notes

- Map keys use RFC 8949 §4.2.1 bytewise order.
- Tags, `undefined`, and unassigned simple values are rejected.
- Zero runtime dependencies (the `derive` feature adds build-time proc-macro dependencies only).
