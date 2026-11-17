use tpt_cbor::Encode;

#[derive(Encode)]
struct S(#[cbor(skip_serializing_if = "is_zero")] u8);

fn is_zero(v: &u8) -> bool {
    *v == 0
}

fn main() {}
