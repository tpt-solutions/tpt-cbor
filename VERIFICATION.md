# Verification status

Status: **Kani: all ten component harnesses verified.** **Miri: the core
integration tests pass on Linux.** Other checks were run on Windows with the
pinned toolchain, Rust 1.85.0, unless the row says otherwise.

## Kani (formal proofs)

Kani 0.68.0 runs in WSL (Ubuntu 26.04), installed with the owner's approval.
The Kani toolchain is nightly-2026-08-21, set up by `cargo kani setup`. Kani does
not run on Windows, so these results come from WSL. CI runs the same harnesses
on Ubuntu, one job each.

### Harnesses (`crates/tpt-cbor/src/proofs.rs`)

| Harness | Claim | Result | Time (WSL) |
|---------|-------|--------|------------|
| `header_parse_never_panics` | Header parsing and classification never panic for any first byte | verified | 1–3 s |
| `integer_arguments_never_panic` | Integer readers (`decode_u64`, `decode_i64`) never panic or read out of bounds on a 9-byte input | verified | 2 s |
| `string_decode_never_panics` | String and byte-string readers never panic on a 9-byte input (UTF-8 modelled) | verified | 2 s |
| `float_decode_never_panics` | The float reader never panics on a 9-byte input | verified | 1 s |
| `container_headers_never_panic` | Array and map length readers never panic or read out of bounds | verified | 2 s |
| `check_key_order_never_panics` | The key-order check never panics for any spans | verified | 1 s |
| `unsigned_encoding_minimal_and_round_trips` | Unsigned integer encoding is minimal and round-trips for all `u64` | verified | 7 s |
| `signed_round_trips` | Signed integer encoding round-trips for all `i64` | verified | 10 s |
| `float_canonical_round_trip` | Strict decode and re-encode of any non-NaN `f64` gives identical bytes | verified | 11 s |
| `container_depth_never_exceeds_limit` | Container nesting never exceeds the configured limit for any sequence of entries and exits; a refused entry leaves the depth unchanged. Bounds `skip_item`'s recursion | verified | 1 s |

The solver was also tried: `cargo kani --solver cadical` on all ten harnesses
verified every one, in 0–11 s each, close to the default solver's times. The
default solver is kept.

### Not proved: `skip_item` as a whole

`skip_item` itself is not proved. Four attempts did not finish within the
limits. Each was run on its own, on the same machine:

| Attempt | Limit | Outcome |
|---------|-------|---------|
| One harness, all major types, 8-byte input, depth 2 | 25 min | timed out |
| Split by major type, 8-byte input | 15 min each | scalars, arrays and maps each timed out |
| Split by major type, 4-byte input | 15 min | array timed out |
| Contract stub for `check_key_order`, 4-byte input, depth 2 | 20 min | timed out |
| Contract stub, depth 1 | 20 min | timed out |

The cost comes from CBMC encoding the map key-order code (a slice comparison
with unbounded loops) in every harness that reaches `skip_item`. Stubbing that
code did not bring the time down enough, so `skip_item` is covered by the parts
it is built from. The proved parts are header parsing, integer and string
readers, the float reader, container headers and key order, all listed above.
Its own recursion is covered by the unit tests (`depth_limit_is_enforced`,
`malformed.rs`) and the fuzzer. CI does not run the `skip_item` attempts.

### Optimisations made to the Kani work

* **UTF-8 modelled under Kani.** `from_utf8` unwinds its loop once per byte of
  every symbolic string. Under `cfg(kani)` the check is an arbitrary accept or
  reject. Panic and bounds proofs do not depend on string contents. The tests
  and the fuzzer still cover real validation.
* **Fixed-width `read_be`.** The big-endian reader is now a match on the four
  widths the decoder uses. A loop whose bound depends on the input is gone. The
  decoder gets the same benefit, and the change also sped up the proofs.
* **Piece-wise harnesses.** Small harnesses replace the whole-function proof.
  Each runs in seconds, so CI runs them in parallel.

Before these changes, the five original harnesses took 7–14 s each for the round
trips, and `skip_item_bounded` did not finish within 25 minutes. The ten
harnesses above take about 40 s together in total.

## Miri (undefined behaviour)

Miri runs on the nightly toolchain. CI runs it on Ubuntu, weekly and on demand.
The full core integration test set (`cargo miri test -p tpt-cbor --all-features
--tests`) passes on Linux, in about 1 minute of wall time after setup, with the
malformed-input sweep taking about 90 seconds of that.

To keep Miri fast, the heavy loops scale down under `cfg(miri)`. The exhaustive
second-byte sweep steps through every 17th value. The random-input sweep runs
500 cases instead of 50,000. Two large-input tests use 2,000 items or depth
instead of 100,000. Each scaled test still reaches the branch it exists for:
depth above the hard ceiling, and containers above the pre-allocation cap.

A Miri run on Windows reported undefined behaviour in `zero_alloc.rs`. The report
was in the standard library's Windows allocator, called from the test harness's
own cleanup. The same test passes on Linux, and CI runs Miri on Linux. The
Windows report is a Windows-hosted Miri artefact, not a decoder bug.

## Benchmarks

The benchmark crate (`bench/`) is criterion-only. It benchmarks tpt-cbor's typed
encode and decode, the `Value` path, and zero-allocation validation. The
ciborium and minicbor comparisons are left out until the dependency policy is
decided. Its dependencies pass `cargo deny`.

## Test and build checks

| Check | Command | Result |
|-------|---------|--------|
| Workspace tests, all features | `cargo test --workspace --all-features` | pass |
| Workspace tests, default | `cargo test --workspace` | pass |
| Workspace tests, `std` | `cargo test --workspace --features std` | pass |
| Workspace tests, no default features | `cargo test --workspace --no-default-features` | pass |
| Appendix A vectors | `crates/tpt-cbor/tests/rfc8949_appendix_a.rs` | 12 pass |
| Phase 7 behaviour | `crates/tpt-cbor/tests/ergonomics.rs` | 17 pass |
| Derive behaviour and attributes | `cargo test -p tpt-cbor-derive-tests` | 15 + 13 pass |
| Derive compile-fail | same crate, `tests/ui/` | 11 cases pass (each expected error matches its `.stderr`) |
| CLI end-to-end | `cargo test -p tpt-cbor-cli` | 18 pass |
| Clippy, `-D warnings`, all targets | `cargo clippy --workspace --all-features --all-targets -- -D warnings` | pass |
| Clippy, core without default features | `cargo clippy -p tpt-cbor --lib --no-default-features -- -D warnings` | pass |
| rustfmt | `cargo fmt --all -- --check` | pass |
| Rustdoc, warnings are errors | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | pass |
| `no_std` build, thumbv7em | `cargo build -p tpt-cbor --no-default-features --target thumbv7em-none-eabihf` | pass |
| Core has no dependencies, dev included | `cargo tree -p tpt-cbor --edges normal,build,dev` | only `tpt-cbor` |
| Examples | `canonical_sign`, `hostile_input`, `no_std_embedded` (both feature sets) | run as expected |

## Fuzzing

Fuzzing runs on Windows MSVC with nightly. libFuzzer needs the ASan runtime
DLL on `PATH` (`clang_rt.asan_dynamic-x86_64.dll`, shipped with the Visual
Studio C++ build tools). Without it the binary exits with `STATUS_DLL_NOT_FOUND`.

| Run | Command | Result |
|-----|---------|--------|
| Smoke, before Phase 7 | `cargo +nightly fuzz run decode -- -max_total_time=60` | 519,810 runs, no crashes |
| Long, before Phase 7 | `cargo +nightly fuzz run decode -- -max_total_time=300` | 1,388,221 runs, no crashes |
| Smoke, after Phase 7 and seeded from `fuzz/seeds/decode` | `cargo +nightly fuzz run decode -- -max_total_time=60` | 191,279 runs, no crashes |

The seed corpus (`fuzz/seeds/decode`, 167 files) was generated from the hex
inputs in the test suite, using the CLI's `hex-decode`.

## Licence and source policy

| Check | Result |
|-------|--------|
| `cargo deny check` (root workspace) | advisories, bans, licences and sources pass |
| `cargo deny --manifest-path fuzz/Cargo.toml check` | all four pass |
| `cargo deny` on the benchmark crate | **fails**: ciborium is Apache-2.0 only, and minicbor is BlueOak-1.0.0. The benchmark crate is not in the repository, so this is not checked in CI. |

The allow list gained `NCSA`, for `libfuzzer-sys`, which is a development
dependency of the fuzz harness only.

## Publishing (no upload was made)

| Check | Result |
|-------|--------|
| `cargo publish --dry-run -p tpt-cbor-derive` | passes |
| `cargo publish --dry-run -p tpt-cbor` | cannot run until `tpt-cbor-derive` is on crates.io, because `tpt-cbor` depends on it |

Publish order: `tpt-cbor-derive`, then `tpt-cbor`. `tpt-cbor-cli` and
`tpt-cbor-derive-tests` are `publish = false`.

## Not run

* `skip_item` as a whole, under Kani (see the Kani section for the attempts).
* Minimum dependency versions, coverage, and cargo-semver-checks. These have CI
  jobs, written but not run here. The semver check needs a published baseline,
  so it can only pass after the first release.
* The scheduled 30-minute fuzz job. The 300-second local run above covers the
  same harness.

## Appendix A caveat

The Appendix A rows were transcribed from the RFC text, which was fetched and
summarised by a tool, not copied byte for byte. Re-check the hex against the
RFC before relying on those rows as a reference.

## Limits

* The proofs cover the decoder's pieces: header, integer, string, float,
  container-header and key-order readers, plus the encoder's integer and float
  round trips. They do not cover `skip_item` as a whole (see the Kani section),
  nor the `Vec`, `String` and `Value` allocation paths. Those are covered by tests
  and the fuzz harness.
* Under Kani, UTF-8 validation is modelled as an arbitrary accept or reject. The
  panic and bounds proofs do not depend on string contents.
* Unwinding bounds are chosen by hand (10 for every harness). Each harness
  checks its own bound, so a bound that is too small shows up as a failure.
* Allocation capacity is capped (`PREALLOC_LIMIT`), and each container is
  bounded by `Config::max_items`. The total number of items in a document is
  not bounded, so memory use is still proportional to input size times the
  per-element size.
