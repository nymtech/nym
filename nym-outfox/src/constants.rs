pub const GROUPELEMENTBYTES: u8 = 32;
pub const TAGBYTES: u8 = 16;
pub const MIX_PARAMS_LEN: usize = 6;
pub const MIN_MESSAGE_LEN: usize = 24 * 2;
pub(crate) const CONTEXT: &str = "LIONKEYS";
pub(crate) const TAG_LEN: usize = 24;
pub const DEFAULT_ROUTING_INFO_SIZE: u8 = 32;
pub const DEFAULT_HOPS: usize = 4;

pub const OUTFOX_PACKET_OVERHEAD: usize = MIX_PARAMS_LEN
    + (groupelementbytes() + tagbytes() + DEFAULT_ROUTING_INFO_SIZE as usize) * DEFAULT_HOPS;

pub const fn groupelementbytes() -> usize {
    GROUPELEMENTBYTES as usize
}

pub const fn tagbytes() -> usize {
    TAGBYTES as usize
}
