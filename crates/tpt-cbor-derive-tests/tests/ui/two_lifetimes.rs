use tpt_cbor::Decode;

#[derive(Decode)]
struct S<'a, 'b> {
    a: &'a str,
    b: &'b str,
}

fn main() {}
