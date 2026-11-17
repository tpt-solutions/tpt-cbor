# tpt-cbor — Project Todo

Owner: TPT Solutions · License: MIT OR Apache-2.0 · Policy: zero dependencies in core; no Apache-only (or any non-permissive) dependencies anywhere.

## Decisions locked in
- [x] Derive crate uses `syn`/`quote` (both MIT OR Apache-2.0); core crate stays zero-dep
- [x] Decoder strict by default, opt-in lenient mode; indefinite lengths always rejected
- [x] `Value::Integer(i64)` only; out-of-range ints, tags, unknown simple values -> `DecodeError`
- [x] No SIMD in v1.0; use `core::str::from_utf8`; 100% safe Rust
- [x] Map key order = RFC 8949 §4.2.1 bytewise lexicographic (spec §4 amended)
- [x] Cargo workspace, edition 2021, pinned MSRV, GitHub Actions CI, publish to crates.io
- [x] std I/O: `Encoder` -> `io::Write`; decode via read-to-buffer helper, then zero-copy

---

## Phase 0 — Project Setup
- [x] `git init`, add `.gitignore` (target/, Cargo.lock policy for libs) — repo already initialised; `.gitignore` covers target/, Cargo.lock, fuzz output
- [x] Create Cargo workspace (`tpt-cbor`, `tpt-cbor-derive` placeholder)
- [x] Pin edition 2021 and choose/pin MSRV (`rust-version`) + `rust-toolchain.toml` — MSRV 1.85, pinned in `rust-toolchain.toml`
- [x] Add `LICENSE-MIT` (copyright TPT Solutions)
- [x] Add `LICENSE-APACHE` (Apache-2.0 text, TPT Solutions) — verbatim Apache-2.0 text
- [x] Set `license = "MIT OR Apache-2.0"` in every Cargo.toml
- [x] Add license section + contribution clause (dual-license inbound=outbound) to README
- [x] Write README (goals, feature flags, quick example)
- [x] Add `deny.toml` (cargo-deny): allow only permissive licenses, reject Apache-only deps, ban unexpected deps in core — config written; `cargo-deny` not run locally (not installed)
- [x] CI: build/test on stable + MSRV, `no_std` build (thumbv7em target), clippy, rustfmt, cargo-deny — workflow written; not yet run on GitHub
- [x] Crate-level lints: `#![no_std]`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`
- [x] Clippy deny-list in core (CI-enforced): `indexing_slicing`, `unwrap_used`, `expect_used`, `panic`, `arithmetic_side_effects` — clean with `-D warnings`
- [x] Amend spec.txt §4 (bytewise map order), §7 (drop SIMD claim, branchless wording) and §5.3 to match decisions
- [ ] Reserve crate names on crates.io (`tpt-cbor`, `tpt-cbor-derive`, `tpt-cbor-serde`) — **needs you**: requires a crates.io account and publishing; not done

## Phase 1 — Core Substrate (`tpt-cbor`, zero deps)
### Errors & config
- [x] `DecodeError` (incl. `DepthExceeded`, `Overflow`, `UnexpectedEof`, `InvalidUtf8`, `IndefiniteLength`, `NonCanonical`, `UnsortedMap`, `DuplicateKey`, `UnsupportedType`, `TrailingBytes`)
- [x] `EncodeError`
- [x] `Display` + `core::error::Error` impls (std `Error` behind feature if MSRV requires)
- [x] Decode config: `strict` (default) vs `lenient`; max depth (default 64)
### Decoder (zero-copy, flat, panic-free)
- [x] `Decoder<'a>` struct (input, offset, depth)
- [x] Header parse: major type + additional info (branchless-style bit ops) — bit-mask parsing; no branch-free claim (spec §7 amended)
- [x] Argument decode with `checked_add` / `ok_or(Overflow)`; no `unwrap`/`expect`/indexing panics
- [x] Minimal-integer check (strict) for all argument widths
- [x] `decode_u64` / `decode_i64` (error on out-of-range)
- [x] `decode_bytes() -> &'a [u8]` (zero-copy)
- [x] `decode_str() -> &'a str` (zero-copy, UTF-8 via `core::str::from_utf8`)
- [x] Arrays + maps with definite lengths; length-vs-remaining-input sanity check (no huge pre-allocation)
- [x] Map key ordering + duplicate detection (strict)
- [x] Floats: half/single/double decode; canonical-form check (strict)
- [x] Bool / null; reject undefined, unknown simple values, tags with clear errors
- [x] Reject indefinite-length items (always)
- [x] Depth tracking and `DepthExceeded`
- [x] `peek` / `finish` (trailing-bytes check) helpers
- [x] Non-allocating `skip_item`/`validate` pass (flat, no `Vec`/`String`); decode-to-`Value` builds on it
### Encoder (strict canonical)
- [x] `Encoder` writing to `Vec<u8>` (alloc) — via `Sink` trait (`Vec<u8>` impl)
- [x] Minimal integer encoding
- [x] Canonical floats: smallest lossless width; canonical quiet NaN (`0xf97e00`); ±0, ±inf
- [x] Definite-length bytes/text/array/map only
- [x] Map encoding sorted by encoded-key bytewise order (RFC 8949 §4.2.1)
- [x] Bool / null
### Traits & Value
- [x] `Encode` and `Decode<'a>` traits
- [x] Impls: ints (u8–u64, i8–i64), bool, `f32`/`f64`, `&str`, `String`, `&[u8]`, `Vec<T>`, `Option<T>`, arrays, tuples — `Vec<u8>` encodes as an array; use `&[u8]` for byte strings
- [x] `Value<'a>` enum (`Integer`, `Bytes`, `Text`, `Array`, `Map`, `Float`, `Bool`, `Null`)
- [x] `Value` encode (sort map keys canonically) and decode
- [x] `Value` helper accessors (`as_str`, `as_i64`, `get`, etc.)
### Features
- [x] `default` = core + alloc; verify `no_std` + `alloc` builds — verified: no-default, alloc, std, thumbv7em
- [x] `std` feature: `Encoder` over `io::Write`
- [x] `std` feature: read-to-buffer helper for `io::Read` -> zero-copy decode
### Testing & quality
- [ ] RFC 8949 Appendix A test vectors (decode + canonical re-encode where applicable) — *partial*: ~70 vectors covering all major types; full Appendix A not yet transcribed and checked against the RFC text
- [x] Round-trip tests (unit + property-style, without external deps) — deterministic PRNG generator, 5,000 cases
- [x] Malformed-input corpus: truncated, bad lengths, bad UTF-8, indefinite, deep nesting, overflow
- [x] Exhaustive header tests: all 256 first bytes, and each with every 1–2 argument-byte combination — all 2-byte inputs, plus argument-width sweep
- [x] Determinism test: identical data -> identical bytes
- [x] Lenient-mode tests
- [x] Zero-allocation-on-decode test (counting allocator)
- [ ] Fuzz harness (cargo-fuzz kept in a separate, non-published dir); short smoke run in every-PR CI, long runs nightly — *written* in `fuzz/` (asserts strict-decode ⇒ canonical re-encode); **not run** (needs nightly + libFuzzer); CI job added
- [x] Benchmarks (dev-only, kept out of core dependency graph) — dependency-free `benches/codec.rs`; coarse numbers only
- [x] Rustdoc with examples; doc-tests pass — crate, `Value` examples; more item-level docs possible
- [x] Confirm `cargo tree` shows zero dependencies

## Phase 2 — Derive Macros (`tpt-cbor-derive`)
- [ ] Crate setup (`proc-macro = true`), deps: `syn`, `quote`, `proc-macro2` only (license-checked) — placeholder crate exists; derive deps not yet added
- [ ] `#[derive(Encode)]` for structs (named, tuple, unit)
- [ ] `#[derive(Decode)]` for structs with borrowed fields (`'a` lifetime handling)
- [ ] Enum support (unit, tuple, struct variants) with defined wire representation
- [ ] Generics and where-clause handling
- [ ] Field attributes (rename, skip, default, etc. — define the minimal set)
- [ ] Deterministic field/map-key ordering in generated encoders
- [ ] `derive` feature re-export from `tpt-cbor`
- [ ] Compile-fail tests (trybuild or equivalent permissively licensed tool)
- [ ] Round-trip tests for derived types
- [ ] Docs and examples

## Phase 3 — Formal Verification (Kani, narrowed)
Cheaper layers (clippy deny-list, exhaustive tests, fuzzing) are in Phases 0–1; Kani only proves what they can't.
- [ ] Install Kani; add `kani` cfg harness module
- [ ] `cfg(kani)`: shrink max depth to 2–3; stub `core::str::from_utf8` with `kani::any`
- [ ] Harness: header parse never panics for any bytes
- [ ] Harness: argument read + offset/length arithmetic never overflows or reads out of bounds
- [ ] Harness: `skip_item` never panics, never reads out of bounds (8–16 byte symbolic input, per-harness `#[kani::unwind]`)
- [ ] Harness: recursion depth bounded by max depth
- [ ] Harness: encoder output for ints/floats is minimal (symbolic value)
- [ ] Reuse proofs via `stub_verified` / contracts where possible
- [ ] Try `#[kani::solver(cadical)]`; record timings
- [ ] Keep `Vec`/`String`/`Value` code out of proofs (covered by fuzzing + tests)
- [ ] Kani CI job runs nightly / on release branches only
- [ ] Document proved properties and limits in `VERIFICATION.md`

## Phase 4 — Ecosystem Bridge (Optional: `tpt-cbor-serde`)
- [ ] Decide go/no-go
- [ ] Crate setup (depends on `serde` — license-check, MIT OR Apache-2.0)
- [ ] `Serializer` using canonical encoder
- [ ] `Deserializer` using zero-copy decoder
- [ ] Tests against serde derive types
- [ ] Docs; keep core crate untouched and serde-free

## Phase 5 — Release
- [ ] Full `cargo-deny` audit (licenses, advisories, sources)
- [ ] Verify MSRV and `no_std` builds in CI — MSRV and no_std verified locally; CI config written, not yet run
- [ ] API review (naming, error types, semver surface)
- [x] `CHANGELOG.md`
- [ ] Final README/docs polish; docs.rs metadata
- [ ] `cargo publish --dry-run` for each crate — done for `tpt-cbor` (no upload). `tpt-cbor-derive` is a placeholder and must not be published yet
- [ ] Publish `tpt-cbor`, then `tpt-cbor-derive`
- [ ] Tag `v1.0.0` and create release notes

## Later / Deferred
- [ ] Optional isolated `simd` feature for UTF-8 validation (only if benchmarks justify; separate unsafe audit)
- [ ] Tag support and wider integer range (i128 / full CBOR int range)
- [ ] Full incremental streaming decoder (owned data)
