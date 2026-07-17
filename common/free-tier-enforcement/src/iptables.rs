// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Small `iptables` / `ip6tables` command builders shared by the free-tier managers
//! (the `tc` classify chain here, the `NYM-GARDEN` chain in task 5).

use crate::command::CommandSpec;
use std::net::IpAddr;

/// An IP address family and its `iptables` binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpTablesFamily {
    V4,
    V6,
}

impl IpTablesFamily {
    pub(crate) const ALL: [IpTablesFamily; 2] = [IpTablesFamily::V4, IpTablesFamily::V6];

    pub(crate) fn binary(self) -> &'static str {
        match self {
            IpTablesFamily::V4 => "iptables",
            IpTablesFamily::V6 => "ip6tables",
        }
    }

    /// Whether `addr` belongs to this family (so a v4 rule uses `iptables`, v6 uses
    /// `ip6tables`).
    pub(crate) fn matches(self, addr: &IpAddr) -> bool {
        match self {
            IpTablesFamily::V4 => addr.is_ipv4(),
            IpTablesFamily::V6 => addr.is_ipv6(),
        }
    }

    fn make_cmd(&self, table: &str, op: &str, chain: &str, extra: &[&str]) -> CommandSpec {
        let mut args = vec![
            "-t".to_string(),
            table.to_string(),
            op.to_string(),
            chain.to_string(),
        ];
        args.extend(extra.iter().map(|s| s.to_string()));
        CommandSpec {
            program: self.binary().to_string(),
            args,
        }
    }

    pub(crate) fn new_chain(&self, table: &str, chain: &str) -> CommandSpec {
        self.make_cmd(table, "-N", chain, &[])
    }

    pub(crate) fn flush_chain(&self, table: &str, chain: &str) -> CommandSpec {
        self.make_cmd(table, "-F", chain, &[])
    }

    pub(crate) fn delete_chain(&self, table: &str, chain: &str) -> CommandSpec {
        self.make_cmd(table, "-X", chain, &[])
    }

    pub(crate) fn append(&self, table: &str, chain: &str, extra: &[&str]) -> CommandSpec {
        self.make_cmd(table, "-A", chain, extra)
    }

    pub(crate) fn delete(&self, table: &str, chain: &str, extra: &[&str]) -> CommandSpec {
        self.make_cmd(table, "-D", chain, extra)
    }
}

impl From<IpAddr> for IpTablesFamily {
    fn from(addr: IpAddr) -> Self {
        match addr {
            IpAddr::V4(_) => IpTablesFamily::V4,
            IpAddr::V6(_) => IpTablesFamily::V6,
        }
    }
}
