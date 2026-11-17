use tpt_cbor::Encode;

#[derive(Encode)]
struct S {
    #[cbor(bytes)]
    a: u32,
}

fn main() {}
