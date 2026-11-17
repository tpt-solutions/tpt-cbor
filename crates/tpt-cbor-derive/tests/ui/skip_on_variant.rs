use tpt_cbor::Encode;

#[derive(Encode)]
enum E {
    #[cbor(skip)]
    A,
}

fn main() {}
