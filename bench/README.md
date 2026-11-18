# Benchmarks

Criterion benchmarks for tpt-cbor: derived encode and decode, the dynamic
`Value` path, and zero-allocation validation, all on one small record.

```sh
cd bench
cargo bench
```

This directory is its own workspace, so its dependencies never change the main
workspace's graph. Only criterion is a dependency. The dependency policy in
`deny.toml` is checked for this crate in CI.

## Reading the results

* Numbers depend on the machine and the load. Compare runs on one machine, and
  use them to catch regressions.
* Comparisons with other CBOR crates are not included yet. ciborium is
  Apache-2.0 only, and minicbor is BlueOak-1.0.0, so both need a decision on
  the dependency policy before they can be added.
