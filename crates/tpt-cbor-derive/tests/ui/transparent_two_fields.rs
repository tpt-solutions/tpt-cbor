use tpt_cbor::Encode;

#[derive(Encode)]
#[cbor(transparent)]
struct S {
    a: u8,
    b: u8,
}

fn main() {}
