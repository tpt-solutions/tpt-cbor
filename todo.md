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
- [x] Create Cargo workspace (`tpt-cbor`, `tpt-cbor-derive`)
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
- [x] Crate setup (`proc-macro = true`), deps: `syn`, `quote`, `proc-macro2` only (license-checked) — `tpt-cbor-derive` with syn 2 / quote / proc-macro2 (build-time only)
- [x] `#[derive(Encode)]` for structs (named, tuple, unit)
- [x] `#[derive(Decode)]` for structs with borrowed fields (`'a` lifetime handling) — one lifetime parameter supported
- [x] Enum support (unit, tuple, struct variants) with defined wire representation — externally tagged: unit = text name; others = {"Name": payload}
- [x] Generics and where-clause handling — type params bounded; at most one lifetime
- [x] Field attributes (rename, skip, default, etc. — minimal set: rename, skip, default (Option fields default to None)
- [x] Deterministic field/map-key ordering in generated encoders — keys sorted at compile time by encoded bytes
- [x] `derive` feature re-export from `tpt-cbor` — opt-in; default build stays dependency-free
- [ ] Compile-fail tests (trybuild or equivalent permissively licensed tool) — deferred: not added to keep the dev-dependency graph small; macro errors are reported via syn::Error
- [x] Round-trip tests for derived types
- [x] Docs and examples — format docs, `value_roundtrip` and `derive_config` examples

## Phase 3 — Formal Verification (Kani, narrowed)
Cheaper layers (clippy deny-list, exhaustive tests, fuzzing) are in Phases 0–1; Kani only proves what they can't.
- [ ] Install Kani; add `kani` cfg harness module — harnesses written (`src/proofs.rs`). Also updated for the new `Config::max_items` field. **Blocked**: Kani's installer needs Unix (no native Windows support). The WSL route was denied by the permission check, so it is not installed. Needs a Linux machine/CI runner or an approved WSL install; see VERIFICATION.md
- [ ] `cfg(kani)`: shrink max depth to 2–3; stub `core::str::from_utf8` with `kani::any` — depth shrink done in `skip_item_bounded` only; the `from_utf8` stub is **not implemented**
- [x] Harness: header parse never panics for any bytes — written, not run
- [x] Harness: argument read + offset/length arithmetic never overflows or reads out of bounds — partly covered (unsigned encoding minimal/round-trip); written, not run
- [x] Harness: `skip_item` never panics, never reads out of bounds (8–16 byte symbolic input, per-harness `#[kani::unwind]`) — written, not run
- [ ] Harness: recursion depth bounded by max depth — not written as a Kani harness; covered by `malformed.rs` tests
- [x] Harness: encoder output for ints/floats is minimal (symbolic value) — written, not run
- [ ] Reuse proofs via `stub_verified` / contracts where possible
- [ ] Try `#[kani::solver(cadical)]`; record timings
- [ ] Keep `Vec`/`String`/`Value` code out of proofs (covered by fuzzing + tests)
- [ ] Kani CI job runs nightly / on release branches only
- [x] Document proved properties and limits in `VERIFICATION.md` — status file written; results pending Kani run

## Phase 4 — Ecosystem Bridge (Optional: `tpt-cbor-serde`)
- [x] Decide go/no-go — **no-go for v1.0** (recommended): keeps core serde-free; bridge deferred
- [ ] Crate setup (depends on `serde` — deferred (no-go for v1.0)
- [ ] `Serializer` using canonical encoder — deferred
- [ ] `Deserializer` using zero-copy decoder — deferred
- [ ] Tests against serde derive types — deferred
- [x] Docs; keep core crate untouched and serde-free — core stays serde-free

## Phase 5 — Release
- [x] Full `cargo-deny` audit (licenses, advisories, sources) — advisories, bans, licences, sources all pass
- [ ] Verify MSRV and `no_std` builds in CI — MSRV and no_std verified locally; CI config written, not yet run
- [ ] API review (naming, error types, semver surface) — partial. Open points: `Vec<u8>` encodes as an array (documented); `DecodeError`/`EncodeError` are `non_exhaustive`; `Config` fields are public
- [x] `CHANGELOG.md`
- [ ] Final README/docs polish; docs.rs metadata — docs.rs metadata set; README updated for derive and verification status
- [ ] `cargo publish --dry-run` for each crate — done for `tpt-cbor` (no upload). `tpt-cbor-derive` must be published after `tpt-cbor`, not before
- [ ] Publish `tpt-cbor`, then `tpt-cbor-derive`
- [ ] Tag `v1.0.0` and create release notes

## Later / Deferred
- [ ] Optional isolated `simd` feature for UTF-8 validation (only if benchmarks justify; separate unsafe audit)
- [ ] Tag support and wider integer range (i128 / full CBOR int range)
- [ ] Full incremental streaming decoder (owned data)

## Phase 6 — Review follow-up: fix stale claims and bugs
- [x] `todo.md` Phase 3: the "cfg(kani) shrink max depth / stub from_utf8" claim corrected (Phase 3 line updated: the from_utf8 stub is not implemented)
- [x] Fix "placeholder" wording for the derive crate in `todo.md` and `CHANGELOG.md`; add derive entries to the changelog
- [x] Add the `derive` feature to the feature table in `lib.rs`
- [x] Make the README Quick start a real doctest — `crates/tpt-cbor/README.md` is compiled as doctests (feature `alloc`)
- [x] Align `spec.txt` §3 (no_std default) and §7 (zero-alloc claim for derived structs) with reality
- [x] Clean the duplicated `check_key_order` doc comment in `decode.rs`
- [x] `check_key_order` span failure reports `Malformed`, not `UnexpectedEof`
- [x] Cap `Vec::with_capacity` pre-allocation in `decode.rs` and `value.rs` (`PREALLOC_LIMIT` = 1024); hostile-header regression tests in `malformed.rs`
- [ ] Add `Config` limits (`max_items` / `max_alloc`) enforced in container decode — **partial**: `max_items` done (`DecodeError::LimitExceeded`, default 2^20 per container). `max_alloc` **deferred** (recommendation: do not add yet). The allocation amplification it would guard against is already capped by the 1024-slot pre-allocation limit and the per-container `max_items`. Revisit if fuzzing or real workloads show total memory growing past input size
- [x] Derive: remove leftover `_assert_ident`; duplicate variant names after `rename` are a compile error; type params used only in skipped fields get no bound; docs.rs metadata added (keywords and categories were already set)
- [x] Make `[T; N]` Encode/Decode feature gating symmetric — **decided: accept the asymmetry.** `Decode` for `[T; N]` stays `alloc`-gated and `Encode` stays available without `alloc`. Stable Rust (1.85) cannot fill a `[T; N]` from fallible decodes without `unsafe` (forbidden) or a `Default` bound, and gating `Encode` would remove no_std array encoding. Documented in `decode.rs`
- [x] Implement `Debug` for `Decoder` and `Encoder` (decoder omits input bytes)
- [x] Finish and verify RFC 8949 Appendix A vectors — `crates/tpt-cbor/tests/rfc8949_appendix_a.rs` (12 tests, every row category). Caveat: the rows came from a tool-summarised fetch of the RFC, so re-check the hex against the RFC text
- [x] trybuild compile-fail tests for the derive — `trybuild` added as a dev-dependency of `tpt-cbor-derive` only (core stays zero-dep). Six cases in `crates/tpt-cbor-derive/tests/ui/` cover duplicate variant names, duplicate keys, unions, two lifetimes, `skip` on a variant, and `rename` on a tuple field. trybuild is pinned to 1.0.100 because newer releases need rustc 1.88 (MSRV is 1.85). Licences checked by hand: every crate in its tree is MIT, or has an MIT alternative, or is Unicode-3.0. `cargo deny` is not installed, so it was not run
- [ ] Run tests, clippy, fuzz (and Kani if installable); record real results in `VERIFICATION.md` — tests, clippy, rustfmt and no_std build are run and recorded. Fuzz (needs nightly + libFuzzer) and Kani (needs Linux/WSL) are **not run**

## Phase 7 — Ergonomics
- [ ] `Value`: `From` impls, `Index<&str>` / `Index<usize>`, `Display` as diagnostic notation (RFC 8949 §8), owned/`into_owned` support, `cbor!{}` literal macro
- [ ] More codec impls: `usize`/`isize`, `char`, `()`, `Box<T>`, `Cow`, `&T`, `BTreeMap`, `HashMap` (std, sorted on encode), tuples up to 8
- [ ] `ByteBuf` newtype and `#[tpt(bytes)]` derive attribute for byte-string fields
- [ ] Derive attributes: `rename_all`, `deny_unknown_fields`, `transparent`, `skip_serializing_if`, `with`
- [ ] Error byte offsets (keep `DecodeError` variants `Copy`)
- [ ] Helpers: `from_reader`, `to_vec_canonical`, `is_canonical`, `canonicalize`, `validate`, RFC 8742 sequence iterator
- [ ] `prelude` module

## Phase 8 — CLI (`tpt-cbor-cli`)
- [ ] New workspace crate with binary `tpt-cbor`
- [ ] Subcommands: hex<->cbor, `diag`, `validate [--strict]`, `canon`, json<->cbor
- [ ] stdin/stdout and file IO; exit codes and offsets on validation failure
- [ ] Keep the `zero-deps` CI job scoped to `tpt-cbor`; document `cargo install` in the README

## Phase 9 — Adoption tooling
- [ ] CI: rustdoc `-D warnings`, `cargo semver-checks`, `cargo publish --dry-run` for all crates, miri, MSRV job, minimal-versions, coverage, Kani job, scheduled long fuzz with corpus cache, `derive` in the test matrix, deny covering `fuzz/`
- [ ] Release automation (release-plz or cargo-release), tag-driven publish workflow, dependabot
- [ ] `CONTRIBUTING.md`, issue/PR templates, `rustfmt.toml`, `clippy.toml`
- [ ] Docs: README comparison vs ciborium/minicbor/serde_cbor, migration guide, cookbook, no_std/embedded guide
- [ ] Examples: `no_std_embedded`, `canonical_sign`, `hostile_input`, CLI usage
- [ ] Commit a fuzz seed corpus
- [ ] Criterion benchmarks against ciborium and minicbor in a separate bench crate
- [ ] Reserve crates.io names (`tpt-cbor`, `tpt-cbor-derive`, `tpt-cbor-cli`, `tpt-cbor-serde`)
