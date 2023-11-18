use nym_service_providers_common::interface;

pub type IpPacketRouterRequest = interface::Request<TaggedIpPacket>;
pub type IpPacketRouterResponse = interface::Response<IpPacket>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TaggedIpPacket {
    pub packet: bytes::Bytes,
    pub return_address: nym_sphinx::addressing::clients::Recipient,
    pub return_mix_hops: Option<u8>,
    pub return_mix_delays: Option<f64>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IpPacket {
    pub packet: bytes::Bytes,
}

impl TaggedIpPacket {
    pub fn from_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(&message.message)
    }
}

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
