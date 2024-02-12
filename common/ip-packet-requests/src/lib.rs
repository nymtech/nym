pub mod codec;
pub mod request;
pub mod response;

pub const CURRENT_VERSION: u8 = 3;

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
