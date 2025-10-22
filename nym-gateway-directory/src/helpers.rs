// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn split_ips(ips: Vec<IpAddr>) -> (Vec<Ipv4Addr>, Vec<Ipv6Addr>) {
    ips.into_iter()
        .fold((vec![], vec![]), |(mut v4, mut v6), ip| {
            match ip {
                IpAddr::V4(ipv4_addr) => v4.push(ipv4_addr),
                IpAddr::V6(ipv6_addr) => v6.push(ipv6_addr),
            }
            (v4, v6)
        })
}
