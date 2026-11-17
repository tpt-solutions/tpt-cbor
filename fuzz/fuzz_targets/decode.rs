#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_cbor::{from_slice, from_slice_with_config, to_vec, Config, Decoder, Value};

fuzz_target!(|data: &[u8]| {
    // Strict decode of arbitrary bytes must never panic.
    if let Ok(value) = from_slice::<Value<'_>>(data) {
        // Strict acceptance implies canonical form, so re-encoding must
        // reproduce the input exactly. This is the core canonicality property.
        let reencoded = to_vec(&value).expect("re-encode of decoded value");
        assert_eq!(reencoded.as_slice(), data, "strict decode accepted non-canonical bytes");
    }

    // Lenient and zero-allocation paths must not panic either.
    let _ = from_slice_with_config::<Value<'_>>(data, Config::lenient());
    let mut decoder = Decoder::new(data);
    let _ = decoder.skip_item();
});
