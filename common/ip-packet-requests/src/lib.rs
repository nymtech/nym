#[derive(serde::Serialize, serde::Deserialize)]
pub struct TaggedIpPacket {
    pub packet: bytes::Bytes,
    pub return_address: nym_sphinx::addressing::clients::Recipient,
    pub return_mix_hops: Option<u8>,
    // pub return_mix_delays: Option<f64>,
}

impl TaggedIpPacket {
    pub fn new(
        packet: bytes::Bytes,
        return_address: nym_sphinx::addressing::clients::Recipient,
        return_mix_hops: Option<u8>,
    ) -> Self {
        TaggedIpPacket {
            packet,
            return_address,
            return_mix_hops,
        }
    }

    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(&message.message)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }
}

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
