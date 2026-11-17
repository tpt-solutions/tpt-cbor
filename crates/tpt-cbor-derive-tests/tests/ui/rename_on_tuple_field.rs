use tpt_cbor::Encode;

#[derive(Encode)]
struct T(#[cbor(rename = "x")] u8);

fn main() {}
