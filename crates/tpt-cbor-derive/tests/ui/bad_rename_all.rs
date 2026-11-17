use tpt_cbor::Encode;

#[derive(Encode)]
#[cbor(rename_all = "shouty")]
struct S {
    a: u8,
}

fn main() {}
