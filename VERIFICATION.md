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

## Limits

* Proofs cover integer and float paths and bounded skipping only. They do not
  cover `Vec`, `String` or `Value` allocation paths. Those are covered by tests
  and the fuzz harness.
* Recursion is bounded by shrinking the depth to 2 for the proof. The general
  depth-bound property relies on the test suite (`malformed.rs`).
* Unwinding bounds are chosen by hand and may need raising for the solver.
