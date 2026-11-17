//! Behaviour of the derive macros, exercised through `tpt_cbor`.

use tpt_cbor::{from_slice, to_vec, Decode, DecodeError, Encode};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn named_struct_round_trips_with_sorted_keys() {
    // Keys "x" (61 78) and "y" (61 79) are already in bytewise order.
    let p = Point { x: 1, y: -2 };
    let bytes = to_vec(&p).unwrap();
    assert_eq!(bytes, hex("a2 61 78 01 61 79 21"));
    assert_eq!(from_slice::<Point>(&bytes).unwrap(), p);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct SortOrder {
    // Declared out of canonical order on purpose. "bb" sorts after "a"
    // bytewise, and the derive must sort at compile time.
    bb: u8,
    a: u8,
    #[cbor(rename = "long-key-name")]
    z: u8,
}

#[test]
fn keys_are_emitted_in_canonical_order_regardless_of_declaration() {
    let v = SortOrder { bb: 2, a: 1, z: 3 };
    let bytes = to_vec(&v).unwrap();
    // Encoded keys: "a" (61 61), "bb" (62 62 62), "long-key-name" (6d ...).
    // Shorter-first is not what bytewise gives; it is 61 < 62 < 6d.
    assert_eq!(&bytes[..4], &hex("a3 61 61 01")[..]);
    assert_eq!(from_slice::<SortOrder>(&bytes).unwrap(), v);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct WithOptions {
    id: u32,
    note: Option<String>,
    #[cbor(default)]
    retries: u8,
    #[cbor(skip)]
    cache: u64,
}

#[test]
fn option_and_default_fields_tolerate_missing_keys() {
    // Only "id" is present. "note" is an Option, so it becomes None. "retries"
    // is marked default, so it becomes 0.
    let bytes = hex("a1 62 69 64 07");
    let v: WithOptions = from_slice(&bytes).unwrap();
    assert_eq!(
        v,
        WithOptions {
            id: 7,
            note: None,
            retries: 0,
            cache: 0
        }
    );
}

#[test]
fn skipped_fields_are_not_encoded() {
    let v = WithOptions {
        id: 1,
        note: Some("hi".into()),
        retries: 2,
        cache: 99,
    };
    let bytes = to_vec(&v).unwrap();
    // Three keys: id, note, retries. "cache" must not appear.
    assert_eq!(bytes[0], 0xa3);
    assert!(!bytes.windows(5).any(|w| w == b"cache"));
    let back: WithOptions = from_slice(&bytes).unwrap();
    assert_eq!(back.cache, 0);
    assert_eq!(back.note.as_deref(), Some("hi"));
}

#[test]
fn missing_required_field_is_an_error() {
    // "y" is missing and is not optional.
    let bytes = hex("a1 61 78 01");
    assert_eq!(from_slice::<Point>(&bytes), Err(DecodeError::MissingField));
}

#[test]
fn unknown_keys_are_ignored() {
    // Extra key "z" with a nested value that must be skipped correctly.
    let bytes = hex("a3 61 78 01 61 79 02 61 7a 82 01 02");
    assert_eq!(from_slice::<Point>(&bytes).unwrap(), Point { x: 1, y: 2 });
}

#[test]
fn duplicate_and_unsorted_keys_are_rejected_in_strict_mode() {
    let dup = hex("a2 61 78 01 61 78 02");
    assert_eq!(from_slice::<Point>(&dup), Err(DecodeError::DuplicateKey));
    let unsorted = hex("a2 61 79 02 61 78 01");
    assert_eq!(
        from_slice::<Point>(&unsorted),
        Err(DecodeError::UnsortedMap)
    );
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Pair(u8, String);

#[derive(Debug, PartialEq, Encode, Decode)]
struct Unit;

#[test]
fn tuple_and_unit_structs() {
    let p = Pair(9, "ok".to_string());
    let bytes = to_vec(&p).unwrap();
    assert_eq!(bytes, hex("82 09 62 6f 6b"));
    assert_eq!(from_slice::<Pair>(&bytes).unwrap(), p);

    assert_eq!(to_vec(&Unit).unwrap(), hex("80"));
    assert_eq!(from_slice::<Unit>(&hex("80")).unwrap(), Unit);
    assert_eq!(
        from_slice::<Unit>(&hex("81 00")),
        Err(DecodeError::TypeMismatch)
    );
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum Command<'a> {
    Stop,
    #[cbor(rename = "go")]
    Go(u8, &'a str),
    Move {
        dx: i8,
        dy: i8,
    },
}

#[test]
fn enum_variants_round_trip() {
    let cases = [
        (Command::Stop, hex("64 53 74 6f 70")),
        (
            Command::Go(3, "fast"),
            hex("a1 62 67 6f 82 03 64 66 61 73 74"),
        ),
        (
            Command::Move { dx: 1, dy: -1 },
            hex("a1 64 4d 6f 76 65 a2 62 64 78 01 62 64 79 20"),
        ),
    ];
    for (value, bytes) in cases {
        assert_eq!(to_vec(&value).unwrap(), bytes, "encode {value:?}");
        assert_eq!(
            from_slice::<Command<'_>>(&bytes).unwrap(),
            value,
            "decode {value:?}"
        );
    }
}

#[test]
fn unknown_enum_variant_is_an_error() {
    assert_eq!(
        from_slice::<Command<'_>>(&hex("63 52 75 6e")), // "Run"
        Err(DecodeError::UnknownVariant)
    );
    assert_eq!(
        from_slice::<Command<'_>>(&hex("a1 63 52 75 6e 80")),
        Err(DecodeError::UnknownVariant)
    );
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Wrapper<T> {
    inner: T,
    list: Vec<T>,
}

#[test]
fn generic_types_get_trait_bounds() {
    let w = Wrapper {
        inner: 5u16,
        list: vec![1, 2],
    };
    let bytes = to_vec(&w).unwrap();
    assert_eq!(from_slice::<Wrapper<u16>>(&bytes).unwrap(), w);
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Borrowed<'a> {
    name: &'a str,
    data: &'a [u8],
}

#[test]
fn borrowed_fields_decode_zero_copy() {
    let v = Borrowed {
        name: "tpt",
        data: &[1, 2, 3],
    };
    let bytes = to_vec(&v).unwrap();
    let back: Borrowed<'_> = from_slice(&bytes).unwrap();
    assert_eq!(back, v);
    // The decoded text points into the input buffer.
    let start = bytes.as_ptr() as usize;
    let end = start + bytes.len();
    let p = back.name.as_ptr() as usize;
    assert!(p >= start && p < end, "name must borrow from input");
}

#[derive(Debug, PartialEq, Encode, Decode)]
struct Nested<'a> {
    point: Point,
    tags: Vec<Command<'a>>,
}

#[test]
fn nested_derived_types() {
    let v = Nested {
        point: Point { x: -1, y: 0 },
        tags: vec![Command::Stop, Command::Go(1, "a")],
    };
    let bytes = to_vec(&v).unwrap();
    assert_eq!(from_slice::<Nested<'_>>(&bytes).unwrap(), v);
}
