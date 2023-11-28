// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::request_filter::allowed_hosts::host::Host;
use ipnetwork::IpNetwork;
use std::collections::HashSet;
use std::net::IpAddr;

/// A simpled grouped set of hosts.
/// It ignores any port information.
#[derive(Debug)]
pub(crate) struct HostsGroup {
    pub(super) domains: HashSet<String>,
    pub(super) ip_nets: HashSet<IpNetwork>,
}

impl HostsGroup {
    pub(crate) fn new(raw_hosts: Vec<Host>) -> HostsGroup {
        let mut domains = HashSet::new();
        let mut ip_nets = HashSet::new();

        for host in raw_hosts {
            match host {
                Host::Domain(domain) => {
                    domains.insert(domain);
                }
                Host::IpNetwork(ipnet) => {
                    ip_nets.insert(ipnet);
                }
            }
        }

        HostsGroup { domains, ip_nets }
    }

    pub(crate) fn contains_domain(&self, host: &str) -> bool {
        self.domains.contains(&host.to_string())
    }

    pub(super) fn contains_ip_address(&self, address: IpAddr) -> bool {
        for ip_net in &self.ip_nets {
            if ip_net.contains(address) {
                return true;
            }
        }

        false
    }

    pub(super) fn contains_ip_network(&self, network: IpNetwork) -> bool {
        self.ip_nets.contains(&network)
    }

    pub(super) fn add_ipnet<N: Into<IpNetwork>>(&mut self, network: N) {
        self.ip_nets.insert(network.into());
    }

    pub(super) fn add_domain(&mut self, domain: &str) {
        self.domains.insert(domain.to_string());
    }
}
