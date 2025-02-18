use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};

pub mod codec;
pub mod sign;
pub mod v6;
pub mod v7;
pub mod v8;

// version 3: initial version
// version 4: IPv6 support
// version 5: Add severity level to info response
// version 6: Increase the available IPs
// version 7: Add signature support (for the future)
// version 8: Anonymous sends

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IpPair {
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
}

impl IpPair {
    pub fn new(ipv4: Ipv4Addr, ipv6: Ipv6Addr) -> Self {
        IpPair { ipv4, ipv6 }
    }
}

impl Display for IpPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "IPv4: {}, IPv6: {}", self.ipv4, self.ipv6)
    }
}

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

fn generate_random() -> u64 {
    use rand::RngCore;
    let mut rng = rand::rngs::OsRng;
    rng.next_u64()
}
