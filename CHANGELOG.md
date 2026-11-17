# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Core codec `tpt-cbor`: strict canonical encoder, strict decoder with opt-in lenient mode, zero-copy borrowing.
- `Encode` / `Decode` traits with impls for integers, floats, bool, text, byte strings, `Vec`, `Option`, arrays and tuples.
- `Value` dynamic type (feature `alloc`), with canonical map ordering and duplicate-key rejection on encode.
- `std` feature: `to_writer`, `IoSink`, `read_to_buffer`.
- Workspace with placeholder `tpt-cbor-derive` crate.
- CI workflow, `deny.toml`, and a `cargo-fuzz` harness under `fuzz/` (outside the workspace).

### Notes

- Map keys use RFC 8949 §4.2.1 bytewise order.
- Tags, `undefined`, and unassigned simple values are rejected.
- Zero runtime dependencies.
