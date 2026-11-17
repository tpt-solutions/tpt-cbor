# Contributing to tpt-cbor

Thank you for your interest in tpt-cbor.

**Contributions are welcome as issues only.** Pull requests are not accepted.
Please open an issue to:

* report a bug, using the bug report form;
* request a feature or a change in behaviour, using the feature request form;
* ask a question or discuss the design.

The maintainers may implement an accepted change themselves. An issue that
describes the problem clearly, with a minimal input or code sample, is the most
useful contribution.

## Before you open an issue

* Check whether a similar issue already exists.
* For a decoding problem, include the input bytes in hex (for example with
  `tpt-cbor hex-encode`) and the exact error you saw.
* For a security problem, do not open a public issue. See
  [SECURITY.md](SECURITY.md).

## Standards the project keeps

These are stated so that issue reports can refer to them.

* The core crate `tpt-cbor` has zero dependencies. A change that adds one will
  be declined.
* Dependencies must be permissively licensed, as checked by `deny.toml`.
* The minimum supported Rust version is 1.85.0, pinned in `rust-toolchain.toml`.
* The core crate forbids `unsafe` code and does not panic on input.
* Encoded output is canonical (RFC 8949 section 4.2.1).

By opening an issue you agree that any code you suggest may be included under
the project's MIT OR Apache-2.0 licence.
