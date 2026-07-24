use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};

pub mod codec;
#[cfg(feature = "test-utils")]
pub mod icmp_utils;
pub mod response_helpers;
pub mod v10;
pub mod v8;
pub mod v9;

/// Connect-failure reason. Defined once in v8 and reused unchanged by v9 and v10,
/// so the shared connect path names it here rather than picking a version.
pub use v8::response::ConnectFailureReason;

/// Highest IPR protocol version that is allowed to be sent as a **non-stream** mixnet payload
/// (i.e. not wrapped in `LpFrameKind::SphinxStream`).
pub const MAX_NON_STREAM_VERSION: u8 = v8::VERSION;

/// First IPR protocol version that **requires** the SphinxStream (LP) transport for non-stream
/// mixnet sends, matching the node-side enforcement in `ip-packet-router`.
pub const SPHINX_STREAM_VERSION_THRESHOLD: u8 = v9::VERSION;

/// Client MTU advertised on mobile/Android, where carrier last-mile caps bite
/// regardless of the IPR.
pub const CLIENT_MTU_MOBILE: u16 = 1280;

/// Client MTU fallback for IPRs that predate MTU negotiation (the v10 connect
/// response); at or below the historic 1420-byte IPR TUN.
pub const CLIENT_MTU_FALLBACK: u16 = 1420;

/// Highest IPR protocol version a node's release supports, from each version's
/// `MIN_RELEASE_VERSION`. Lets a client pick the protocol up front from the
/// node's directory version instead of probing. `None` means the node is too old
/// for even v9.
pub fn best_supported_version(node_version: &semver::Version) -> Option<u8> {
    if *node_version >= v10::MIN_RELEASE_VERSION {
        Some(v10::VERSION)
    } else if *node_version >= v9::MIN_RELEASE_VERSION {
        Some(v9::VERSION)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const _: () = {
        assert!(SPHINX_STREAM_VERSION_THRESHOLD > MAX_NON_STREAM_VERSION);
    };

    #[test]
    fn stream_transport_threshold_is_consistent() {
        assert_eq!(MAX_NON_STREAM_VERSION, 8);
        assert_eq!(SPHINX_STREAM_VERSION_THRESHOLD, 9);
        const _: () = assert!(SPHINX_STREAM_VERSION_THRESHOLD > MAX_NON_STREAM_VERSION);
    }

    #[test]
    fn best_supported_version_ladder() {
        use semver::Version;
        let v = |s: &str| Version::parse(s).unwrap();

        assert_eq!(best_supported_version(&v("1.37.0")), Some(v10::VERSION));
        assert_eq!(best_supported_version(&v("1.37.1")), Some(v10::VERSION));
        assert_eq!(best_supported_version(&v("1.40.0")), Some(v10::VERSION));
        assert_eq!(best_supported_version(&v("1.36.9")), Some(v9::VERSION));
        assert_eq!(best_supported_version(&v("1.30.0")), Some(v9::VERSION));
        assert_eq!(best_supported_version(&v("1.29.9")), None);
    }
}

// Wire-protocol history (v8 is the current floor; v3-v7 removed):
// version 3: initial version
// version 4: IPv6 support
// version 5: Add severity level to info response
// version 6: Increase the available IPs
// version 7: Add signature support (for the future)
// version 8: Anonymous sends
// version 9: LP-framed transport (SphinxStream)
// version 10: IPR reports its accepted MTU in the connect response
// response_helpers: shared IPR response parsing (nym-ip-packet-client + nym-sdk)

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
