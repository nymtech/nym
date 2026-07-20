// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Small `nft` (nftables) command builders shared by the free-tier managers.
//!
//! Each manager owns one `table inet <name>` holding its sets + chain; teardown is a
//! single atomic `delete table`. Native sets give ~O(1) membership without the
//! separate `ipset` package, and the `inet` family means one chain handles v4 and v6
//! (matched by `ip` / `ip6`), so there is no iptables/ip6tables duplication. `nft` is
//! the modern default firewall framework, so it needs no package beyond what current
//! Debian/Ubuntu already ship.
//!
//! Commands are built token-by-token; `nft` joins its argv with spaces and parses the
//! result, so `{`, `}` and `;` are passed as their own tokens.

use crate::command::CommandSpec;

/// The element type of an nft set (an `inet` table still types sets per family).
#[derive(Clone, Copy)]
pub(crate) enum SetType {
    Ipv4,
    Ipv6,
}

impl SetType {
    fn keyword(self) -> &'static str {
        match self {
            SetType::Ipv4 => "ipv4_addr",
            SetType::Ipv6 => "ipv6_addr",
        }
    }
}

pub(crate) fn add_table(table: &str) -> CommandSpec {
    CommandSpec::new("nft", ["add", "table", "inet", table])
}

pub(crate) fn delete_table(table: &str) -> CommandSpec {
    CommandSpec::new("nft", ["delete", "table", "inet", table])
}

/// A plain set of exact addresses (the peer tunnel IPs).
pub(crate) fn add_set(table: &str, set: &str, ty: SetType) -> CommandSpec {
    CommandSpec::new(
        "nft",
        [
            "add",
            "set",
            "inet",
            table,
            set,
            "{",
            "type",
            ty.keyword(),
            ";",
            "}",
        ],
    )
}

/// An interval set (supports ranges / CIDRs), for the whitelist.
pub(crate) fn add_interval_set(table: &str, set: &str, ty: SetType) -> CommandSpec {
    CommandSpec::new(
        "nft",
        [
            "add",
            "set",
            "inet",
            table,
            set,
            "{",
            "type",
            ty.keyword(),
            ";",
            "flags",
            "interval",
            ";",
            "}",
        ],
    )
}

/// A base chain: `type filter hook <hook> priority <priority>; policy <policy>;`.
pub(crate) fn add_chain(
    table: &str,
    chain: &str,
    hook: &str,
    priority: &str,
    policy: &str,
) -> CommandSpec {
    CommandSpec::new(
        "nft",
        [
            "add", "chain", "inet", table, chain, "{", "type", "filter", "hook", hook, "priority",
            priority, ";", "policy", policy, ";", "}",
        ],
    )
}

pub(crate) fn add_rule(table: &str, chain: &str, rule: &[&str]) -> CommandSpec {
    let mut args = vec!["add", "rule", "inet", table, chain];
    args.extend_from_slice(rule);
    CommandSpec::new("nft", args)
}

/// Empty a set (used on whitelist refresh, before repopulating).
pub(crate) fn flush_set(table: &str, set: &str) -> CommandSpec {
    CommandSpec::new("nft", ["flush", "set", "inet", table, set])
}

pub(crate) fn add_element(table: &str, set: &str, addr: &str) -> CommandSpec {
    CommandSpec::new(
        "nft",
        ["add", "element", "inet", table, set, "{", addr, "}"],
    )
}

pub(crate) fn delete_element(table: &str, set: &str, addr: &str) -> CommandSpec {
    CommandSpec::new(
        "nft",
        ["delete", "element", "inet", table, set, "{", addr, "}"],
    )
}
