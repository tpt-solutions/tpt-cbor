use tpt_cbor::Encode;

#[derive(Encode)]
struct S {
    #[cbor(rename = "x")]
    a: u8,
    #[cbor(rename = "x")]
    b: u8,
}

fn main() {}
