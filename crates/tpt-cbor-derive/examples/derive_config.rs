//! Derive `Encode` and `Decode` for a config-like struct and an enum.
//!
//! Run with `cargo run -p tpt-cbor-derive --example derive_config`.

use tpt_cbor::{from_slice, to_vec, Decode, Encode};

#[derive(Debug, PartialEq, Encode, Decode)]
struct Settings {
    name: String,
    #[cbor(rename = "max-retries")]
    max_retries: u8,
    // Missing from older documents: becomes `None` when absent.
    note: Option<String>,
    // Never written to the wire.
    #[cbor(skip)]
    cache_hits: u32,
}

#[derive(Debug, PartialEq, Encode, Decode)]
enum Event {
    Started,
    Progress { percent: u8 },
    Failed(String),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings {
        name: "sync".into(),
        max_retries: 3,
        note: None,
        cache_hits: 42,
    };
    let bytes = to_vec(&settings)?;
    let back: Settings = from_slice(&bytes)?;
    assert_eq!(back.cache_hits, 0, "skipped fields come back as Default");
    println!("settings round-trip: {back:?}");

    for event in [
        Event::Started,
        Event::Progress { percent: 50 },
        Event::Failed("disk".into()),
    ] {
        let bytes = to_vec(&event)?;
        println!("{event:?} -> {bytes:02x?}");
        assert_eq!(from_slice::<Event>(&bytes)?, event);
    }
    Ok(())
}
