//! `std::io` adapter tests (requires feature `std`).
#![cfg(feature = "std")]

use tpt_cbor::{from_slice, read_to_buffer, to_writer, Value};

#[test]
fn to_writer_matches_to_vec() {
    let value = Value::Map(vec![
        (Value::Text("b"), Value::Float(1.5)),
        (Value::Text("a"), Value::Integer(-2)),
    ]);
    let mut out: Vec<u8> = Vec::new();
    to_writer(&value, &mut out).unwrap();
    assert_eq!(out, tpt_cbor::to_vec(&value).unwrap());
}

#[test]
fn read_to_buffer_then_decode() {
    let bytes = tpt_cbor::to_vec(&vec![1u8, 2, 3]).unwrap();
    let buffer = read_to_buffer(bytes.as_slice()).unwrap();
    let decoded: Vec<u8> = from_slice(&buffer).unwrap();
    assert_eq!(decoded, vec![1, 2, 3]);
}

#[test]
fn write_errors_are_reported_with_kind() {
    struct Failing;
    impl std::io::Write for Failing {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let err = to_writer(&Value::Null, Failing).unwrap_err();
    assert_eq!(
        err,
        tpt_cbor::EncodeError::Io(std::io::ErrorKind::BrokenPipe)
    );
}
