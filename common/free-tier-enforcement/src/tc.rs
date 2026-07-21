// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `tc` HTB rate-limit pool for free-tier peers (download direction).
//!
//! A table-scoped builder used by [`crate::FreeTierEnforcement`]: it contributes the
//! HTB qdisc/classes (via `tc`), the pool's peer sets, and a `classify` chain into the
//! shared `nft` table. The chain, on POSTROUTING (which runs before tc egress):
//!   1. source in the shared `allow` set -> `return` (full speed; e.g. purchase endpoints)
//!   2. destination in the pool `peers` set -> `meta priority set 1:10` (the rate-limited pool class)
//!   3. neither -> unshaped, class `1:1` (the HTB default)
//!
//! Membership is a native `nft` set, so per-packet cost is a flat ~O(1) lookup and
//! add/remove is a set-element op. One `inet` chain handles both address families.
//!
//! Ingress (upload) shaping is NOT here yet: netfilter POSTROUTING runs before `tc`
//! egress (so classifying on POSTROUTING works for download), but `tc` ingress runs
//! BEFORE netfilter, so the upload path needs an IFB device + `tc` u32 filters (7.2).

use crate::command::CommandSpec;
use crate::nft::{self, SetType};
use crate::{ALLOW_V4, ALLOW_V6, PeerAddrs, TABLE};

const CHAIN: &str = "classify";
const PEERS_V4: &str = "pool_v4";
const PEERS_V6: &str = "pool_v6";
const POOL_CLASSID: &str = "1:10";
const UNLIMITED_CLASSID: &str = "1:1";
/// Effectively-unlimited ceiling for the default class (unclassified = full speed).
const UNLIMITED_RATE: &str = "10gbit";

/// The rate-limit pool builder (download direction, v1).
#[derive(Clone, Debug)]
pub(crate) struct RateLimitPool {
    interface: String,
    pool_rate_bits_per_sec: u64,
}

impl RateLimitPool {
    pub(crate) fn new(interface: impl Into<String>, pool_bytes_per_sec: u64) -> Self {
        RateLimitPool {
            interface: interface.into(),
            pool_rate_bits_per_sec: pool_bytes_per_sec.saturating_mul(8),
        }
    }

    fn pool_rate(&self) -> String {
        format!("{}bit", self.pool_rate_bits_per_sec)
    }

    /// The HTB structure on the WireGuard interface: root qdisc (unclassified -> `1:1`),
    /// the unlimited default class `1:1`, and the shared pool class `1:10`. Via `tc`.
    pub(crate) fn htb_commands(&self) -> Vec<CommandSpec> {
        let dev = self.interface.as_str();
        let pool_rate = self.pool_rate();
        vec![
            CommandSpec::new(
                "tc",
                [
                    "qdisc", "add", "dev", dev, "root", "handle", "1:", "htb", "default", "1",
                ],
            ),
            CommandSpec::new(
                "tc",
                [
                    "class",
                    "add",
                    "dev",
                    dev,
                    "parent",
                    "1:",
                    "classid",
                    UNLIMITED_CLASSID,
                    "htb",
                    "rate",
                    UNLIMITED_RATE,
                    "ceil",
                    UNLIMITED_RATE,
                ],
            ),
            CommandSpec::new(
                "tc",
                [
                    "class",
                    "add",
                    "dev",
                    dev,
                    "parent",
                    "1:",
                    "classid",
                    POOL_CLASSID,
                    "htb",
                    "rate",
                    pool_rate.as_str(),
                    "ceil",
                    pool_rate.as_str(),
                ],
            ),
        ]
    }

    /// Remove the HTB structure (the shared nft table is deleted by the facade).
    pub(crate) fn htb_teardown_commands(&self) -> Vec<CommandSpec> {
        vec![CommandSpec::new(
            "tc",
            ["qdisc", "del", "dev", self.interface.as_str(), "root"],
        )]
    }

    /// The pool's peer sets in the shared table.
    pub(crate) fn set_commands(&self) -> Vec<CommandSpec> {
        vec![
            nft::add_set(TABLE, PEERS_V4, SetType::Ipv4),
            nft::add_set(TABLE, PEERS_V6, SetType::Ipv6),
        ]
    }

    /// The classify chain (POSTROUTING): whitelist sources are exempt (`return`), then
    /// pooled destinations get the pool tc class. References the shared allow sets.
    pub(crate) fn chain_commands(&self) -> Vec<CommandSpec> {
        let dev = self.interface.as_str();
        let allow4 = &format!("@{ALLOW_V4}");
        let allow6 = &format!("@{ALLOW_V6}");
        let peers4 = &format!("@{PEERS_V4}");
        let peers6 = &format!("@{PEERS_V6}");
        vec![
            nft::add_chain(TABLE, CHAIN, "postrouting", "mangle", "accept"),
            nft::add_rule(
                TABLE,
                CHAIN,
                &["oifname", dev, "ip", "saddr", allow4, "return"],
            ),
            nft::add_rule(
                TABLE,
                CHAIN,
                &["oifname", dev, "ip6", "saddr", allow6, "return"],
            ),
            nft::add_rule(
                TABLE,
                CHAIN,
                &[
                    "oifname",
                    dev,
                    "ip",
                    "daddr",
                    peers4,
                    "meta",
                    "priority",
                    "set",
                    POOL_CLASSID,
                ],
            ),
            nft::add_rule(
                TABLE,
                CHAIN,
                &[
                    "oifname",
                    dev,
                    "ip6",
                    "daddr",
                    peers6,
                    "meta",
                    "priority",
                    "set",
                    POOL_CLASSID,
                ],
            ),
        ]
    }

    pub(crate) fn add_peer_commands(&self, addrs: &PeerAddrs) -> Vec<CommandSpec> {
        vec![
            nft::add_element(TABLE, PEERS_V4, &addrs.v4.to_string()),
            nft::add_element(TABLE, PEERS_V6, &addrs.v6.to_string()),
        ]
    }

    pub(crate) fn remove_peer_commands(&self, addrs: &PeerAddrs) -> Vec<CommandSpec> {
        vec![
            nft::delete_element(TABLE, PEERS_V4, &addrs.v4.to_string()),
            nft::delete_element(TABLE, PEERS_V6, &addrs.v6.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn rendered(cmds: &[CommandSpec]) -> Vec<String> {
        cmds.iter().map(CommandSpec::rendered).collect()
    }

    fn pool() -> RateLimitPool {
        RateLimitPool::new("nymwg", 1_250_000) // 1.25 MB/s -> 10_000_000 bit/s
    }

    const ADDRS: PeerAddrs = PeerAddrs {
        v4: Ipv4Addr::new(10, 1, 0, 5),
        v6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 5),
    };

    #[test]
    fn htb_builds_the_pool_class_at_the_configured_rate() {
        let cmds = rendered(&pool().htb_commands());
        assert!(cmds.contains(
            &"tc class add dev nymwg parent 1: classid 1:10 htb rate 10000000bit ceil 10000000bit"
                .to_string()
        ));
    }

    #[test]
    fn classify_chain_exempts_whitelist_before_classifying_pooled_peers() {
        let cmds = rendered(&pool().chain_commands());
        assert!(
            cmds.contains(
                &"nft add rule inet nym_free_tier classify oifname nymwg ip saddr @allow_v4 return"
                    .to_string()
            )
        );
        assert!(cmds.contains(
            &"nft add rule inet nym_free_tier classify oifname nymwg ip daddr @pool_v4 meta priority set 1:10"
                .to_string()
        ));
        let exempt = cmds
            .iter()
            .position(|c| c.contains("@allow_v4 return"))
            .unwrap();
        let classify = cmds
            .iter()
            .position(|c| c.contains("@pool_v4 meta priority set"))
            .unwrap();
        assert!(exempt < classify);
    }

    #[test]
    fn add_and_remove_peer_touch_the_pool_sets() {
        assert_eq!(
            rendered(&pool().add_peer_commands(&ADDRS)),
            vec![
                "nft add element inet nym_free_tier pool_v4 { 10.1.0.5 }".to_string(),
                "nft add element inet nym_free_tier pool_v6 { fd00::5 }".to_string(),
            ]
        );
        assert_eq!(
            rendered(&pool().remove_peer_commands(&ADDRS)),
            vec![
                "nft delete element inet nym_free_tier pool_v4 { 10.1.0.5 }".to_string(),
                "nft delete element inet nym_free_tier pool_v6 { fd00::5 }".to_string(),
            ]
        );
    }
}
