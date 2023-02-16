use crate::format::MixCreationParameters;

#[allow(dead_code)]
pub struct OutfoxPacket {
    params: MixCreationParameters,
    payload: Vec<u8>,
}
