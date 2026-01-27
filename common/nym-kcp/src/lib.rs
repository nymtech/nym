pub mod codec;
pub mod driver;
pub mod error;
pub mod packet;
pub mod session;

pub const MAX_RTO: u32 = 60000; // Same as used in update_rtt
