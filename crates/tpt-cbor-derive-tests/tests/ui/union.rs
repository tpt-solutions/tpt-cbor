use tpt_cbor::Encode;

#[derive(Encode)]
union U {
    a: u8,
}

fn main() {}
