use libcrux_kem::Algorithm;

pub const GROUPELEMENTBYTES: u8 = 32;
pub const TAGBYTES: u8 = 16;
pub const MIX_PARAMS_LEN: usize = DEFAULT_HOPS + 2;
pub const MIN_MESSAGE_LEN: usize = 24 * 2;
pub(crate) const CONTEXT: &str = "LIONKEYS";
pub(crate) const TAG_LEN: usize = 24;
pub const DEFAULT_ROUTING_INFO_SIZE: u8 = 32;
pub const DEFAULT_HOPS: usize = 4;
pub const ROUTING_INFORMATION_LENGTH_BY_STAGE: [u8; DEFAULT_HOPS] =
    [DEFAULT_ROUTING_INFO_SIZE; DEFAULT_HOPS];
pub const MIN_PACKET_SIZE: usize = 48;
pub const MAGIC_SLICE: &[u8] = &[111, 102, 120];

pub const fn outfox_packet_overhead(kem: Algorithm) -> usize {
    MIX_PARAMS_LEN
        + (groupelementbytes(kem) + tagbytes() + DEFAULT_ROUTING_INFO_SIZE as usize) * DEFAULT_HOPS
        + MAGIC_SLICE.len()
}

pub const fn groupelementbytes(kem: Algorithm) -> usize {
    match kem {
        Algorithm::XWingKemDraft06 => 1120,
        Algorithm::X25519 => 32,
        Algorithm::MlKem768 => 1088,
        _ => unreachable!(),
    }
}

pub const fn tagbytes() -> usize {
    TAGBYTES as usize
}
