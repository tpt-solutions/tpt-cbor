# Verification status

Status: **proof harnesses written, not yet run.**

Kani is not installed. Kani's installer only builds on Unix. A native Windows
install fails with platform errors (`std::os::unix` is not available). Installing
it in WSL was also attempted, and that attempt was blocked by the permission
check, so no Linux install was done. To finish, either approve the WSL install
(rustup, `cargo install kani-verifier --locked`, `cargo kani setup`) or run the
proofs on a Linux machine or CI runner. Then run:

    cargo kani -p tpt-cbor

and record the outcome of each harness below.

## Harnesses (`crates/tpt-cbor/src/proofs.rs`)

| Harness | Claim | Result |
|---------|-------|--------|
| `header_parse_never_panics` | Header parsing and classification never panic for any first byte | not run |
| `skip_item_bounded` | `skip_item` never panics or reads out of bounds on 8-byte input (depth 2) | not run |
| `unsigned_encoding_minimal_and_round_trips` | Unsigned integer encoding is minimal and round-trips for all `u64` | not run |
| `signed_round_trips` | Signed integer encoding round-trips for all `i64` | not run |
| `float_canonical_round_trip` | Strict decode and re-encode of any non-NaN `f64` gives identical bytes | not run |

## Other checks (run on Windows, toolchain 1.85.0)

| Check | Command | Result |
|-------|---------|--------|
| Workspace tests, all features | `cargo test --workspace --all-features` | pass |
| Core tests, no default features | `cargo test -p tpt-cbor --no-default-features` | pass |
| Core tests, `std` | `cargo test -p tpt-cbor --features std` | pass |
| Clippy, `-D warnings`, all targets | `cargo clippy --workspace --all-features --all-targets -- -D warnings` | pass |
| rustfmt | `cargo fmt --all -- --check` | pass |
| `no_std` library build | `cargo build -p tpt-cbor --no-default-features` | pass |
| Derive compile-fail tests | `cargo test -p tpt-cbor-derive --test compile_fail` | pass (6 cases in `crates/tpt-cbor-derive/tests/ui/`) |
| Fuzz harness, 60 s smoke | `cargo +nightly fuzz run decode -- -max_total_time=60` | pass: 519,810 runs, no crashes |
| Fuzz harness, 300 s | `cargo +nightly fuzz run decode -- -max_total_time=300` | see the note below the table |
| Kani proofs | `cargo kani -p tpt-cbor` | **not run**: see above |
| Appendix A vectors | `crates/tpt-cbor/tests/rfc8949_appendix_a.rs` | 12 pass |

Fuzzing runs on Windows MSVC with nightly. libFuzzer needs the ASan runtime
DLL on `PATH` (`clang_rt.asan_dynamic-x86_64.dll`, shipped with the Visual
Studio C++ build tools). Without it the binary exits with `STATUS_DLL_NOT_FOUND`.

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
