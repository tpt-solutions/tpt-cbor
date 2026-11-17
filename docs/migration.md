# Migrating from serde_cbor

This guide lists the differences that most often break a port. It covers
serde-based code. For the wider comparison, see [comparison.md](comparison.md).

## Traits and derives

| serde / serde_cbor | tpt-cbor |
|--------------------|----------|
| `#[derive(Serialize)]` | `#[derive(Encode)]` (feature `derive`) |
| `#[derive(Deserialize)]` | `#[derive(Decode)]` |
| `serde_cbor::to_vec(&v)` | `tpt_cbor::to_vec(&v)` |
| `serde_cbor::from_slice::<T>(b)` | `tpt_cbor::from_slice::<T>(b)` |
| `serde_cbor::Value` | `tpt_cbor::Value` (see the differences below) |
| `#[serde(rename = "x")]` | `#[cbor(rename = "x")]` |
| `#[serde(rename_all = "camelCase")]` | `#[cbor(rename_all = "camelCase")]` |
| `#[serde(skip)]` | `#[cbor(skip)]` |
| `#[serde(default)]` | `#[cbor(default)]` |
| `#[serde(skip_serializing_if = "...")]` | `#[cbor(skip_serializing_if = "...")]` |
| `#[serde(deny_unknown_fields)]` | `#[cbor(deny_unknown_fields)]` |
| `#[serde(transparent)]` | `#[cbor(transparent)]` |
| `#[serde(with = "module")]` | `#[cbor(with = "module")]`, with `encode` and `decode` functions |
| `#[serde(with = "serde_bytes")]` | `#[cbor(bytes)]` on `Vec<u8>` or `&[u8]` |

`#[serde(flatten)]`, `#[serde(untagged)]`, `#[serde(tag = "...")]` and the
other enum representations have no equivalent. Rewrite those types by hand.

## Output differs

* **Field order.** serde_cbor writes struct fields in declaration order.
  tpt-cbor sorts map keys by their encoded bytes (RFC 8949 section 4.2.1), so
  bytes change even for the same data. Anything that compares or hashes the
  encoding must be re-derived once.
* **Byte strings.** serde_cbor writes `serialize_bytes` as a byte string.
  tpt-cbor writes `Vec<u8>` as an array of integers. Use `ByteBuf`,
  `&[u8]`, or `#[cbor(bytes)]`.
* **Floats.** tpt-cbor always writes the shortest lossless width, and NaN is
  always `0xf97e00`.
* **Integers.** Encoding is always minimal.

## Input is rejected more often

The default `Config::strict()` rejects input that serde_cbor accepts:

* non-minimal integers and floats, and non-canonical map key order (use
  `Config::lenient()` to accept them);
* indefinite-length strings, arrays and maps, which are always rejected;
* tags (for example date strings and bignums), `undefined` and unassigned
  simple values, which are always rejected.

If you need tags, decode them in a wrapper before handing the value to
tpt-cbor, or keep using a crate that supports them.

## Dynamic values

| | serde_cbor::Value | tpt_cbor::Value |
|-|-------------------|-----------------|
| integers | a wider range (check serde_cbor's current docs) | `i64` only; larger values fail with `Overflow` |
| maps | ordered map | a `Vec` of pairs, sorted on encode |
| tags | variant | rejected |
| text and bytes | owned | borrowed (`Value<'a>`); use `OwnedValue` for owned data |

## Errors

Errors are `Copy` values such as `DecodeError::NonCanonical`, and they do not
allocate. Use `from_slice_located` to get the byte offset where decoding
stopped.

## A checklist for a port

1. Replace the derives and the functions. Build with `derive` enabled.
2. Decide on canonical or lenient decoding for each input. Use `Config`.
3. Find every `Vec<u8>` that was a byte string and mark it `#[cbor(bytes)]`.
4. Find every tag, indefinite-length item and `undefined` in your data, and
   plan how to handle it.
5. Regenerate any stored or signed encodings. Their bytes will change.
