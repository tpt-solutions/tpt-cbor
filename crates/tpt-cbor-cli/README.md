# tpt-cbor-cli

A command-line tool for CBOR (RFC 8949), built on [`tpt-cbor`](../tpt-cbor).
It converts between binary CBOR, hex, diagnostic notation and JSON, and checks
whether input is valid CBOR.

This crate is not published to crates.io. Install it from a checkout:

```sh
cargo install --path crates/tpt-cbor-cli
```

## Commands

| Command | Input | Output |
|---------|-------|--------|
| `hex-encode` | binary CBOR | lowercase hex text |
| `hex-decode` | hex text (whitespace allowed) | binary CBOR |
| `diag` | binary CBOR | diagnostic notation (RFC 8949 §8) |
| `validate [--strict]` | binary CBOR | nothing; exit status says whether it is valid |
| `canon` | binary CBOR | the same data in canonical form |
| `cbor-to-json` | binary CBOR | JSON |
| `json-to-cbor` | JSON | canonical binary CBOR |

Options: `-i FILE` reads the input from a file, and `-o FILE` writes the output
to one. Without them the tool reads standard input and writes standard output.

`validate` accepts non-canonical input unless `--strict` is given. Strict mode
rejects it, as the library's default `Config::strict()` does.

## Examples

```sh
# {"a": 1, "b": [2, 3]} in CBOR, then back to JSON.
echo '{"a": 1, "b": [2, 3]}' | tpt-cbor json-to-cbor | tpt-cbor hex-encode
# a26161016162820203

echo 'a2 61 61 01 61 62 82 02 03' | tpt-cbor hex-decode | tpt-cbor diag
# {"a": 1, "b": [2, 3]}

# Non-minimal integer 0x18 0x01: lenient is fine, strict is not.
echo '18 01' | tpt-cbor hex-decode | tpt-cbor validate
echo '18 01' | tpt-cbor hex-decode | tpt-cbor validate --strict
# tpt-cbor: invalid CBOR: non-canonical encoding at byte 2
```

## Exit status

| Status | Meaning |
|--------|---------|
| 0 | Success. |
| 1 | The input is not valid for the command. The message gives the byte offset when there is one. |
| 2 | Usage error, or an I/O failure. |

## JSON mapping

* Integers within `i64` become CBOR integers. Other numbers become floats.
* Objects become maps with text keys. Keys are sorted on output, so the
  result is canonical.
* Byte strings, non-text map keys and `NaN` or infinities have no JSON form.
  Converting them fails with status 1.
* JSON nesting deeper than 128 levels is rejected.

## Licence

Dual-licensed under MIT or Apache-2.0, as for the rest of the workspace.
