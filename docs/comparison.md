# Comparison with other Rust CBOR crates

This page describes design differences. It is not a benchmark, and the
repository does not yet contain one (see `todo.md`, Phase 9). Facts about other projects
can change, so check their current documentation before you rely on them.

| | tpt-cbor | ciborium | minicbor | serde_cbor |
|-|----------|----------|----------|------------|
| Built on serde | no | yes | no | yes |
| Own derive macros | yes (`Encode`, `Decode`) | no (serde derives) | yes | no (serde derives) |
| Runtime dependencies (core) | none | several | none in the core | several |
| Output | canonical by design (RFC 8949 section 4.2.1) | as the serializer writes it | as the encoder writes it | as the serializer writes it |
| Decoding strictness | strict by default; lenient opt-in | lenient | lenient | lenient |
| Tags | rejected | supported | supported | supported |
| Indefinite lengths | rejected | accepted | accepted | accepted |
| Zero-copy decoding | yes (`&str`, `&[u8]`) | limited | yes | limited |
| `no_std` | yes, with or without `alloc` | yes | yes | no |
| Unsafe code in the core | forbidden | forbidden | not used by the core | not used |
| Maintenance status | active, pre-release | active | active | archived by its author (check) |
| Licence | MIT OR Apache-2.0 | MIT OR Apache-2.0 | BlueOak-1.0.0 | MIT OR Apache-2.0 |

## When to choose tpt-cbor

* You need the same bytes for the same data, for hashing or signing.
* You want to reject non-canonical or unexpected input by default.
* You want no serde, and no dependencies in the core crate.
* You are writing for `no_std` targets and want a small, auditable core.

## When another crate fits better

* You need tags, indefinite-length items, or data that must round-trip
  unchanged. Use ciborium or minicbor.
* Your types already derive serde and you do not want a second derive.
  Use ciborium.
* You need the full range of CBOR integers (`i128`-sized values) or tags in a
  dynamic value. tpt-cbor's `Value` is limited to `i64`.

## Notes on the table

* "Zero-copy" means borrowing text and bytes from the input. It is not
  decoding without allocation for every type.
* minicbor's licence is BlueOak-1.0.0, which is not in `deny.toml`'s allow
  list. tpt-cbor's core does not depend on it.
* ciborium's licence is Apache-2.0 alone. The project policy rejects
  Apache-only dependencies, so a comparison benchmark that uses ciborium needs
  a policy decision first.
