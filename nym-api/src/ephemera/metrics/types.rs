#[derive(Debug)]
pub struct MixnodeResult {
    pub mix_id: u32,
    pub reliability: u8,
}

// value in range 0-100
#[derive(Clone, Copy, Debug, Default)]
pub struct Uptime(u8);
