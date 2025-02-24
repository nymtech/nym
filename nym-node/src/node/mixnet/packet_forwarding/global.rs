// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// use `ip` feature without nightly
// issue: https://github.com/rust-lang/rust/issues/27709
pub(crate) const fn is_global_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(addr) => is_global_ipv4(addr),
        IpAddr::V6(addr) => is_global_ipv6(addr),
    }
}

const fn is_shared_ipv4(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 100 && (ip.octets()[1] & 0b1100_0000 == 0b0100_0000)
}

const fn is_benchmarking_ipv4(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] == 198 && (ip.octets()[1] & 0xfe) == 18
}

const fn is_reserved_ipv4(ip: &Ipv4Addr) -> bool {
    ip.octets()[0] & 240 == 240 && !ip.is_broadcast()
}

const fn is_global_ipv4(ip: &Ipv4Addr) -> bool {
    !(ip.octets()[0] == 0 // "This network"
        || ip.is_private()
        || is_shared_ipv4(ip)
        || ip.is_loopback()
        || ip.is_link_local()
        // addresses reserved for future protocols (`192.0.0.0/24`)
        // .9 and .10 are documented as globally reachable so they're excluded
        || (
        ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0
            && ip.octets()[3] != 9 && ip.octets()[3] != 10
    )
        || ip.is_documentation()
        || is_benchmarking_ipv4(ip)
        || is_reserved_ipv4(ip)
        || ip.is_broadcast())
}

const fn is_documentation_ipv6(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] == 0x2001) && (ip.segments()[1] == 0xdb8)
}

const fn is_global_ipv6(ip: &Ipv6Addr) -> bool {
    !(ip.is_unspecified()
        || ip.is_loopback()
        // IPv4-mapped Address (`::ffff:0:0/96`)
        || matches!(ip.segments(), [0, 0, 0, 0, 0, 0xffff, _, _])
        // IPv4-IPv6 Translat. (`64:ff9b:1::/48`)
        || matches!(ip.segments(), [0x64, 0xff9b, 1, _, _, _, _, _])
        // Discard-Only Address Block (`100::/64`)
        || matches!(ip.segments(), [0x100, 0, 0, 0, _, _, _, _])
        // IETF Protocol Assignments (`2001::/23`)
        || (matches!(ip.segments(), [0x2001, b, _, _, _, _, _, _] if b < 0x200)
        && !(
        // Port Control Protocol Anycast (`2001:1::1`)
        u128::from_be_bytes(ip.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0001
            // Traversal Using Relays around NAT Anycast (`2001:1::2`)
            || u128::from_be_bytes(ip.octets()) == 0x2001_0001_0000_0000_0000_0000_0000_0002
            // AMT (`2001:3::/32`)
            || matches!(ip.segments(), [0x2001, 3, _, _, _, _, _, _])
            // AS112-v6 (`2001:4:112::/48`)
            || matches!(ip.segments(), [0x2001, 4, 0x112, _, _, _, _, _])
            // ORCHIDv2 (`2001:20::/28`)
            // Drone Remote ID Protocol Entity Tags (DETs) Prefix (`2001:30::/28`)`
            || matches!(ip.segments(), [0x2001, b, _, _, _, _, _, _] if b >= 0x20 && b <= 0x3F)
    ))
        // 6to4 (`2002::/16`) – it's not explicitly documented as globally reachable,
        // IANA says N/A.
        || matches!(ip.segments(), [0x2002, _, _, _, _, _, _, _])
        || is_documentation_ipv6(ip)
        || ip.is_unique_local()
        || ip.is_unicast_link_local())
}
