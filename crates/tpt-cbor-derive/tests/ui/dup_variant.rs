use tpt_cbor::{Decode, Encode};

#[derive(Encode, Decode)]
enum E {
    A,
    #[cbor(rename = "A")]
    B,
}

fn main() {}
