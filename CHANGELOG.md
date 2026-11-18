# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Core codec `tpt-cbor`: strict canonical encoder, strict decoder with opt-in lenient mode, zero-copy borrowing.
- `Encode` / `Decode` traits with impls for integers, floats, bool, text, byte strings, `Vec`, `Option`, arrays and tuples.
- `Value` dynamic type (feature `alloc`), with canonical map ordering and duplicate-key rejection on encode.
- `std` feature: `to_writer`, `IoSink`, `read_to_buffer`.
- `tpt-cbor-derive` crate: `#[derive(Encode)]` and `#[derive(Decode)]` for structs and enums, re-exported behind the `derive` feature of `tpt-cbor`.
- `Config::max_items` and `DecodeError::LimitExceeded`: containers longer than the limit are rejected before their elements are read.
- `Debug` for `Decoder` and `Encoder`. The decoder's `Debug` omits the input bytes.
- CI workflow, `deny.toml`, and a `cargo-fuzz` harness under `fuzz/` (outside the workspace).
- `From` impls for `Value`, `Index<&str>` and `Index<usize>` (missing keys give `null`), and `Display` in RFC 8949 diagnostic notation.
- `cbor!` macro for building `Value` literals, and `OwnedValue` with `Value::into_owned`.
- `ByteBuf` for owned byte strings.
- Codecs for `usize`, `isize`, `char`, `()`, `Box`, `Cow`, `BTreeMap`, `HashMap` (feature `std`), `[u8]`, and tuples up to arity 8.
- `Located` errors (a `DecodeError` plus its byte offset), with `from_slice_located` and `from_slice_located_with_config`.
- `validate`, `is_canonical`, `canonicalize`, `sequence` (RFC 8742), `from_reader` (feature `std`), and a `prelude` module.
- `EncodeError::BufferFull` and `DecodeError::UnknownField`.
- Derive attributes: `rename_all`, `deny_unknown_fields`, `transparent`, `skip_serializing_if`, `with`, and `bytes`.
- `tpt-cbor-cli`: the `tpt-cbor` command-line tool (`hex-encode`, `hex-decode`, `diag`, `validate`, `canon`, `cbor-to-json`, `json-to-cbor`). Not published.
- RFC 8949 Appendix A tests, and trybuild compile-fail tests for the derive macros.
- Examples `canonical_sign`, `hostile_input` and `no_std_embedded`, and documentation in `docs/`.
- Seed corpus for the fuzz harness in `fuzz/seeds/decode`.
- Community files: `CONTRIBUTING.md` (issues only), `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue forms, dependabot, `rustfmt.toml`, `clippy.toml`, and a manual release-PR workflow with `publish = false`.
- Kani proof `container_depth_never_exceeds_limit`, bounding container nesting. Ten Kani harnesses now verify, one CI job each, and Miri runs the core integration tests on Linux.

### Changed

- Decoding checks container length against `Config::max_items` before elements are read.
- The workspace uses resolver 3, which the 1.85 MSRV supports.
- `check_key_order` reports `Malformed` for a span outside the input.
- The derive test suite moved to `tpt-cbor-derive-tests`, so `tpt-cbor-derive` no longer depends on `tpt-cbor`. Publishing order is `tpt-cbor-derive`, then `tpt-cbor`.
- `value_roundtrip` requires the `alloc` feature.

### Notes

- Map keys use RFC 8949 §4.2.1 bytewise order.
- Tags, `undefined`, and unassigned simple values are rejected.
- Zero runtime dependencies.
