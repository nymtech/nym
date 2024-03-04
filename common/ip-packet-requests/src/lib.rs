use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};

pub mod codec;
pub mod request;
pub mod response;

// version 3: initial version
// version 4: IPv6 support
pub const CURRENT_VERSION: u8 = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IPPair {
    pub ipv4: Ipv4Addr,
    pub ipv6: Ipv6Addr,
}

impl IPPair {
    pub fn new(ipv4: Ipv4Addr, ipv6: Ipv6Addr) -> Self {
        IPPair { ipv4, ipv6 }
    }
}

impl Display for IPPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "IPv4: {}, IPV6: {}", self.ipv4, self.ipv6)
    }
}

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
