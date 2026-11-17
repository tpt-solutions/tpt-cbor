# tpt-cbor documentation

* [Cookbook](cookbook.md): short recipes for common tasks.
* [No-std and embedded](no_std.md): using the crate without `alloc`.
* [Migrating from serde_cbor](migration.md): differences that matter when
  moving code from a serde-based CBOR crate.
* [Comparison with other crates](comparison.md): how tpt-cbor differs from
  ciborium, minicbor and serde_cbor.
* [Verification status](../VERIFICATION.md): which checks have been run.
* [Specification notes](../spec.txt): the design rules the crate follows.

Runnable examples are in `crates/tpt-cbor/examples/`:

| Example | Shows |
|---------|-------|
| `value_roundtrip` | Building, encoding and decoding a dynamic `Value`. |
| `derive_config` (in `tpt-cbor-derive-tests`) | Derived types and field attributes. |
| `canonical_sign` | Two insertion orders give the same bytes. |
| `hostile_input` | Explicit limits and error offsets for untrusted input. |
| `no_std_embedded` | The no-allocation path, with a custom fixed-size `Sink`. |

The command-line tool is documented in [`crates/tpt-cbor-cli`](../crates/tpt-cbor-cli/README.md).
