//! Container and field attributes added in Phase 7: `rename_all`,
//! `deny_unknown_fields`, `transparent`, `skip_serializing_if`, `with` and
//! `bytes`.

use tpt_cbor::{from_slice, to_vec, Decode, DecodeError, Decoder, Encode, EncodeError, Encoder};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

/// Canonical bytes of a short text string (length below 24).
fn text(s: &str) -> Vec<u8> {
    assert!(s.len() < 24);
    let mut out = vec![0x60 | s.len() as u8];
    out.extend_from_slice(s.as_bytes());
    out
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(rename_all = "camelCase")]
struct CamelFields {
    first_name: u8,
    last_name: u8,
}

#[test]
fn rename_all_camel_case_on_fields() {
    let v = CamelFields {
        first_name: 1,
        last_name: 2,
    };
    // "lastName" (8 bytes) sorts before "firstName" (9 bytes): its first byte is smaller.
    let mut expected = hex("a2");
    expected.extend(text("lastName"));
    expected.push(0x02);
    expected.extend(text("firstName"));
    expected.push(0x01);
    assert_eq!(to_vec(&v).unwrap(), expected);
    assert_eq!(from_slice::<CamelFields>(&expected).unwrap(), v);
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(rename_all = "snake_case")]
enum Mode {
    FastPath,
    Slow(u8),
    #[cbor(rename = "custom")]
    Other,
}

#[test]
fn rename_all_applies_to_variant_names_and_rename_wins() {
    assert_eq!(to_vec(&Mode::FastPath).unwrap(), text("fast_path"));
    assert_eq!(to_vec(&Mode::Other).unwrap(), text("custom"));
    let slow = to_vec(&Mode::Slow(7)).unwrap();
    let mut expected = hex("a1");
    expected.extend(text("slow"));
    expected.extend(hex("81 07"));
    assert_eq!(slow, expected);
    assert_eq!(
        from_slice::<Mode>(&text("fast_path")).unwrap(),
        Mode::FastPath
    );
    assert_eq!(from_slice::<Mode>(&slow).unwrap(), Mode::Slow(7));
}

#[test]
fn rename_all_rules_split_words_as_documented() {
    #[derive(Encode)]
    #[cbor(rename_all = "SCREAMING-KEBAB-CASE")]
    struct Words {
        http_server_port: u8,
    }
    let bytes = to_vec(&Words {
        http_server_port: 1,
    })
    .unwrap();
    let mut expected = hex("a1");
    expected.extend(text("HTTP-SERVER-PORT"));
    expected.push(0x01);
    assert_eq!(bytes, expected);
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(deny_unknown_fields)]
struct Strict {
    a: u8,
}

#[test]
fn deny_unknown_fields_rejects_extra_keys() {
    // {"a": 1, "b": 2}
    assert_eq!(
        from_slice::<Strict>(&hex("a2 61 61 01 61 62 02")),
        Err(DecodeError::UnknownField)
    );
    assert_eq!(
        from_slice::<Strict>(&hex("a1 61 61 01")).unwrap(),
        Strict { a: 1 }
    );
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Lenient {
    a: u8,
}

#[test]
fn unknown_fields_are_ignored_by_default() {
    assert_eq!(
        from_slice::<Lenient>(&hex("a2 61 61 01 61 62 02")).unwrap(),
        Lenient { a: 1 }
    );
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(deny_unknown_fields)]
enum Event {
    Click { x: u8 },
}

#[test]
fn deny_unknown_fields_applies_to_struct_variants() {
    // {"Click": {"x": 1, "y": 2}}
    assert_eq!(
        from_slice::<Event>(&hex("a1 65 43 6c 69 63 6b a2 61 78 01 61 79 02")),
        Err(DecodeError::UnknownField)
    );
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(transparent)]
struct Id(u64);

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(transparent)]
struct Name {
    inner: String,
}

#[test]
fn transparent_structs_encode_as_their_field() {
    assert_eq!(to_vec(&Id(5)).unwrap(), hex("05"));
    assert_eq!(from_slice::<Id>(&hex("05")).unwrap(), Id(5));
    let name = Name {
        inner: "ab".to_string(),
    };
    assert_eq!(to_vec(&name).unwrap(), hex("62 61 62"));
    assert_eq!(from_slice::<Name>(&hex("62 61 62")).unwrap(), name);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Profile {
    name: String,
    #[cbor(skip_serializing_if = "Option::is_none")]
    nick: Option<String>,
}

#[test]
fn skip_serializing_if_omits_the_key_when_true() {
    let without = Profile {
        name: "a".to_string(),
        nick: None,
    };
    // Only "name": a one-entry map.
    assert_eq!(to_vec(&without).unwrap(), hex("a1 64 6e 61 6d 65 61 61"));
    assert_eq!(
        from_slice::<Profile>(&to_vec(&without).unwrap()).unwrap(),
        without
    );

    let with = Profile {
        name: "a".to_string(),
        nick: Some("b".to_string()),
    };
    // Both keys are 4 characters, so their encodings share the prefix 0x64.
    // "name" (6e 61 6d 65) sorts before "nick" (6e 69 63 6b).
    let mut expected = hex("a2");
    expected.extend(text("name"));
    expected.extend(text("a"));
    expected.extend(text("nick"));
    expected.extend(text("b"));
    let bytes = to_vec(&with).unwrap();
    assert_eq!(bytes, expected);
    assert_eq!(from_slice::<Profile>(&bytes).unwrap(), with);
}

/// A codec for a `u32` written as lowercase hex text.
mod hex_codec {
    use super::*;

    pub fn encode(value: &u32, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_str(&format!("{value:x}"))
    }

    pub fn decode(decoder: &mut Decoder<'_>) -> Result<u32, DecodeError> {
        let text = decoder.decode_str()?;
        u32::from_str_radix(text, 16).map_err(|_| DecodeError::Malformed)
    }
}

/// A type with no codec impls. `with` must not require one.
#[derive(Debug, PartialEq)]
struct Opaque(u8);

mod opaque_codec {
    use super::*;

    pub fn encode(value: &Opaque, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        encoder.encode_u64(u64::from(value.0))
    }

    pub fn decode(decoder: &mut Decoder<'_>) -> Result<Opaque, DecodeError> {
        let v = decoder.decode_u64()?;
        u8::try_from(v)
            .map(Opaque)
            .map_err(|_| DecodeError::Overflow)
    }
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Wrapped {
    #[cbor(with = "hex_codec")]
    value: u32,
    #[cbor(with = "opaque_codec")]
    opaque: Opaque,
}

#[test]
fn with_uses_the_named_module_and_needs_no_codec_impl() {
    let v = Wrapped {
        value: 255,
        opaque: Opaque(9),
    };
    // "value" (0x65...) sorts before "opaque" (0x66...) by its first byte.
    // "ff" is the hex text of 255. The opaque value 9 is a plain integer.
    let mut expected = hex("a2");
    expected.extend(text("value"));
    expected.extend(text("ff"));
    expected.extend(text("opaque"));
    expected.push(0x09);
    let bytes = to_vec(&v).unwrap();
    assert_eq!(bytes, expected);
    assert_eq!(from_slice::<Wrapped>(&bytes).unwrap(), v);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Blob {
    #[cbor(bytes)]
    data: Vec<u8>,
}

#[test]
fn bytes_attribute_writes_a_byte_string() {
    let v = Blob {
        data: vec![1, 2, 3],
    };
    let mut expected = hex("a1");
    expected.extend(text("data"));
    expected.extend(hex("43 01 02 03"));
    assert_eq!(to_vec(&v).unwrap(), expected);
    assert_eq!(from_slice::<Blob>(&expected).unwrap(), v);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Borrowed<'a> {
    #[cbor(bytes)]
    data: &'a [u8],
}

#[test]
fn bytes_attribute_on_borrowed_slice_is_zero_copy() {
    let input = hex("a1 64 64 61 74 61 43 01 02 03");
    let decoded: Borrowed<'_> = from_slice(&input).unwrap();
    assert_eq!(decoded.data, &[1, 2, 3]);
    // The slice points into the input buffer.
    let start = input.as_ptr() as usize;
    let end = start + input.len();
    let p = decoded.data.as_ptr() as usize;
    assert!(p >= start && p < end);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Raw(#[cbor(bytes)] Vec<u8>);

#[test]
fn bytes_attribute_works_on_tuple_fields() {
    let v = Raw(vec![0xff]);
    assert_eq!(to_vec(&v).unwrap(), hex("81 41 ff"));
    assert_eq!(from_slice::<Raw>(&hex("81 41 ff")).unwrap(), v);
}

#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(rename_all = "kebab-case")]
struct Mixed {
    #[cbor(rename = "ID")]
    user_id: u8,
    display_name: u8,
}

#[test]
fn explicit_rename_overrides_rename_all() {
    let v = Mixed {
        user_id: 1,
        display_name: 2,
    };
    let bytes = to_vec(&v).unwrap();
    // Keys: "ID" (2 bytes, 0x62) and "display-name" (12 bytes, 0x6c).
    // Bytewise "ID" is 0x62 0x49 0x44, which sorts first.
    let mut expected = hex("a2");
    expected.extend(text("ID"));
    expected.push(0x01);
    expected.extend(text("display-name"));
    expected.push(0x02);
    assert_eq!(bytes, expected);
    assert_eq!(from_slice::<Mixed>(&bytes).unwrap(), v);
}
