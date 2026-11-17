//! The no-allocation path, for embedded targets.
//!
//! This example uses only the borrowed API: no `Vec`, no `String`, no `Value`.
//! It encodes into a fixed-size buffer through a custom `Sink`, and decodes a
//! value that borrows its text from that buffer. It builds without default
//! features:
//!
//! ```text
//! cargo run -p tpt-cbor --example no_std_embedded --no-default-features
//! ```
//!
//! The example runs on a host, so it uses `std` for printing. The code above
//! `main` depends only on `core`.

use tpt_cbor::{Decode, DecodeError, Decoder, Encode, EncodeError, Encoder, Sink};

/// A sink that writes into a fixed array and reports `BufferFull` when it runs
/// out of room.
struct FixedBuf<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> FixedBuf<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn written(&self) -> &[u8] {
        // `len` never exceeds N, so the range is in bounds.
        self.bytes.get(..self.len).unwrap_or(&[])
    }
}

impl<const N: usize> Sink for FixedBuf<N> {
    fn write_all_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let end = self
            .len
            .checked_add(bytes.len())
            .ok_or(EncodeError::BufferFull)?;
        let dst = self
            .bytes
            .get_mut(self.len..end)
            .ok_or(EncodeError::BufferFull)?;
        dst.copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
}

/// A reading that borrows its label from the input.
#[derive(Debug, PartialEq)]
struct Reading<'a> {
    label: &'a str,
    sensor: u8,
}

impl Encode for Reading<'_> {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), EncodeError> {
        // Keys in RFC 8949 section 4.2.1 order: "label" (0x65...) before
        // "sensor" (0x66...).
        encoder.enter_container()?;
        encoder.encode_map_header(2)?;
        encoder.encode_str("label")?;
        encoder.encode_str(self.label)?;
        encoder.encode_str("sensor")?;
        self.sensor.encode(encoder)?;
        encoder.leave_container();
        Ok(())
    }
}

impl<'a> Decode<'a> for Reading<'a> {
    fn decode(decoder: &mut Decoder<'a>) -> Result<Self, DecodeError> {
        let len = decoder.decode_map_len()?;
        decoder.enter_container()?;
        let mut label: Option<&'a str> = None;
        let mut sensor: Option<u8> = None;
        // Hand-written decoders check key order themselves.
        let mut prev: Option<(usize, usize)> = None;
        for _ in 0..len {
            let start = decoder.position();
            let key = decoder.decode_str()?;
            let span = (start, decoder.position());
            decoder.check_key_order(prev, span)?;
            prev = Some(span);
            match key {
                "label" => label = Some(decoder.decode_str()?),
                "sensor" => sensor = Some(u8::decode(decoder)?),
                _ => decoder.skip_item()?,
            }
        }
        decoder.leave_container();
        Ok(Reading {
            label: label.ok_or(DecodeError::MissingField)?,
            sensor: sensor.ok_or(DecodeError::MissingField)?,
        })
    }
}

fn main() {
    let reading = Reading {
        label: "t",
        sensor: 7,
    };

    // The encoding is 17 bytes, so 32 is comfortably enough.
    let mut buffer = FixedBuf::<32>::new();
    let mut encoder = Encoder::new(&mut buffer);
    reading.encode(&mut encoder).expect("32 bytes is enough");
    let encoded = buffer.written();
    println!(
        "encoded into 32-byte buffer ({} bytes): {encoded:02x?}",
        encoded.len()
    );

    // The decoded label points into `encoded`. Nothing is copied.
    let decoded = Reading::decode(&mut Decoder::new(encoded)).expect("valid encoding");
    assert_eq!(decoded, reading);
    println!("decoded: {decoded:?}");

    // A 4-byte buffer is too small: the sink reports BufferFull.
    let mut small = FixedBuf::<4>::new();
    let result = reading.encode(&mut Encoder::new(&mut small));
    println!("into 4-byte buffer: {result:?}");
}
