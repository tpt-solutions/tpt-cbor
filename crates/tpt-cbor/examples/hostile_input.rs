//! Decoding untrusted bytes.
//!
//! Pick explicit limits for the input, then report failures with their byte
//! offset. Limits are checked before anything large is allocated, so a
//! hostile length header cannot force a big allocation.
//!
//! Run with: `cargo run -p tpt-cbor --example hostile_input`

use tpt_cbor::{from_slice_located_with_config, Config, Value};

fn main() {
    // Nesting at most 8 deep, and at most 16 items in any one container.
    let limits = Config {
        max_depth: 8,
        max_items: 16,
        ..Config::strict()
    };

    // An array of 100 nulls: the header is valid, but 100 > max_items.
    let hundred_nulls: Vec<u8> = [&[0x98, 0x64][..], &[0xf6; 100][..]].concat();

    let cases: &[(&str, &[u8])] = &[
        ("small map", &[0xa1, 0x61, 0x61, 0x01]),
        // Twenty nested one-element arrays: deeper than max_depth.
        ("deep nesting", &[0x81; 20]),
        // An array header claiming 2^64 - 1 elements, with no body.
        (
            "huge length, no body",
            &[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        ("too many items", &hundred_nulls),
        // Text longer than the input: a truncated string.
        ("truncated text", &[0x64, 0x61]),
        // A non-minimal integer, rejected in strict mode.
        ("non-canonical", &[0x18, 0x01]),
    ];

    for (name, bytes) in cases {
        match from_slice_located_with_config::<Value<'_>>(bytes, limits) {
            Ok(value) => println!("{name:>22}: ok, {value}"),
            Err(located) => println!("{name:>22}: rejected, {located}"),
        }
    }
}
