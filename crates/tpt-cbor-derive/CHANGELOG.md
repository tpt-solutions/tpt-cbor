# Changelog

All notable changes to `tpt-cbor-derive` are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `#[derive(Encode)]` and `#[derive(Decode)]` for named, tuple and unit structs, and for externally tagged enums.
- `#[cbor(rename = "...")]`, `#[cbor(skip)]` and `#[cbor(default)]` attributes.

### Fixed

- Type parameters used only by `#[cbor(skip)]` fields no longer receive `Encode` or `Decode` bounds.
- Two enum variants whose names collide after `rename` are a compile error, not an unreachable match arm.
- Removed an unused helper left over from development.

### Notes

- Not intended for direct use. Enable the `derive` feature of `tpt-cbor` instead.
- Build-time dependencies only: `syn`, `quote` and `proc-macro2`. None are linked into `tpt-cbor` itself.
- Not yet published to crates.io. See the work tracker in `todo.md`.
