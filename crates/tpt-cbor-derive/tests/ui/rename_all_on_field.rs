use tpt_cbor::Encode;

#[derive(Encode)]
struct S {
    #[cbor(rename_all = "snake_case")]
    a: u8,
}

fn main() {}
