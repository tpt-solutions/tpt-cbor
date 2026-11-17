# Verification status

Status: **proof harnesses written, not yet run.** Everything else below was run
on Windows with the pinned toolchain, Rust 1.85.0, unless the row says otherwise.

## Kani (formal proofs)

Kani is not installed. Kani's installer only builds on Unix. A native Windows
install fails with platform errors (`std::os::unix` is not available). Installing
it in WSL was also attempted, and that attempt was blocked by the permission
check, so no Linux install was done. To finish, either approve the WSL install
(rustup, `cargo install kani-verifier --locked`, `cargo kani setup`) or run the
proofs on a Linux machine or CI runner. The CI workflow has a weekly and manual
Kani job that runs `cargo kani -p tpt-cbor`. Record the outcome of each harness
below.

### Harnesses (`crates/tpt-cbor/src/proofs.rs`)

| Harness | Claim | Result |
|---------|-------|--------|
| `header_parse_never_panics` | Header parsing and classification never panic for any first byte | not run |
| `skip_item_bounded` | `skip_item` never panics or reads out of bounds on 8-byte input (depth 2) | not run |
| `unsigned_encoding_minimal_and_round_trips` | Unsigned integer encoding is minimal and round-trips for all `u64` | not run |
| `signed_round_trips` | Signed integer encoding round-trips for all `i64` | not run |
| `float_canonical_round_trip` | Strict decode and re-encode of any non-NaN `f64` gives identical bytes | not run |

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

* Kani proofs (see above).
* Miri, minimum dependency versions, coverage, and cargo-semver-checks. These
  have CI jobs, written but not run here. The semver check needs a published
  baseline, so it can only pass after the first release.
* The scheduled 30-minute fuzz job. The 300-second local run above covers the
  same harness.

## Appendix A caveat

The Appendix A rows were transcribed from the RFC text, which was fetched and
summarised by a tool, not copied byte for byte. Re-check the hex against the
RFC before relying on those rows as a reference.

## Limits

* Proofs cover integer and float paths and bounded skipping only. They do not
  cover `Vec`, `String` or `Value` allocation paths. Those are covered by tests
  and the fuzz harness.
* Recursion is bounded by shrinking the depth to 2 for the proof. The general
  depth-bound property relies on the test suite (`malformed.rs`).
* The `skip_item_bounded` harness is the only one with a shrunk depth. The
  planned `from_utf8` stub under `cfg(kani)` is not implemented.
* Unwinding bounds are chosen by hand and may need raising for the solver.
* Allocation capacity is capped (`PREALLOC_LIMIT`), and each container is
  bounded by `Config::max_items`. The total number of items in a document is
  not bounded, so memory use is still proportional to input size times the
  per-element size.
