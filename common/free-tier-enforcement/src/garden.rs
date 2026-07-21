// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `nftables` walled garden for exhausted free-tier peers.
//!
//! A table-scoped builder used by [`crate::FreeTierEnforcement`]: it contributes the
//! garden's peer sets and a `forward`-hook `confine` chain into the shared `nft` table.
//! The chain runs at a low priority (ahead of the operator/iptables filter chains) so
//! the DROP-based garden wins, and lives in the node's own table so it never touches
//! operator rules. Per family, for a confined peer (source in the peer set):
//!   * destination in the shared `allow` set -> `accept` (reach the whitelist, continue
//!     to operator processing);
//!   * everything else -> `drop`.
//!
//! A peer not in the set matches neither rule and falls through untouched. Membership is
//! a native `nft` set, so the per-packet decision is a flat ~O(1) lookup and confining /
//! releasing a peer is a set-element update. One `inet` chain covers both families.

use crate::command::CommandSpec;
use crate::nft::{self, SetType};
use crate::{ALLOW_V4, ALLOW_V6, PeerAddrs, TABLE};

const CHAIN: &str = "confine";
const PEERS_V4: &str = "garden_v4";
const PEERS_V6: &str = "garden_v6";
/// Runs before the iptables/operator filter chains (priority `filter` == 0), so the
/// DROP-based garden takes precedence over any operator ACCEPT.
const CHAIN_PRIORITY: &str = "-1";

/// The walled-garden builder (v1).
#[derive(Clone, Debug)]
pub(crate) struct WalledGarden {
    interface: String,
}

impl WalledGarden {
    pub(crate) fn new(interface: impl Into<String>) -> Self {
        WalledGarden {
            interface: interface.into(),
        }
    }

    /// The garden's peer sets in the shared table.
    pub(crate) fn set_commands(&self) -> Vec<CommandSpec> {
        vec![
            nft::add_set(TABLE, PEERS_V4, SetType::Ipv4),
            nft::add_set(TABLE, PEERS_V6, SetType::Ipv6),
        ]
    }

    /// The confine chain (FORWARD, low priority): a confined peer reaches the whitelist
    /// (`accept`) or is dropped. References the shared allow sets.
    pub(crate) fn chain_commands(&self) -> Vec<CommandSpec> {
        let dev = self.interface.as_str();
        let allow4 = &format!("@{ALLOW_V4}");
        let allow6 = &format!("@{ALLOW_V6}");
        let peers4 = &format!("@{PEERS_V4}");
        let peers6 = &format!("@{PEERS_V6}");
        vec![
            nft::add_chain(TABLE, CHAIN, "forward", CHAIN_PRIORITY, "accept"),
            nft::add_rule(
                TABLE,
                CHAIN,
                &[
                    "iifname", dev, "ip", "saddr", peers4, "ip", "daddr", allow4, "accept",
                ],
            ),
            nft::add_rule(
                TABLE,
                CHAIN,
                &["iifname", dev, "ip", "saddr", peers4, "drop"],
            ),
            nft::add_rule(
                TABLE,
                CHAIN,
                &[
                    "iifname",
                    dev,
                    "ip6",
                    "saddr",
                    peers6,
                    "ip6",
                    "daddr",
                    allow6.as_str(),
                    "accept",
                ],
            ),
            nft::add_rule(
                TABLE,
                CHAIN,
                &["iifname", dev, "ip6", "saddr", peers6.as_str(), "drop"],
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

    fn garden() -> WalledGarden {
        WalledGarden::new("nymwg")
    }

    const ADDRS: PeerAddrs = PeerAddrs {
        v4: Ipv4Addr::new(10, 1, 0, 5),
        v6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 5),
    };

    #[test]
    fn confine_chain_accepts_whitelist_then_drops() {
        let cmds = rendered(&garden().chain_commands());
        assert!(cmds.contains(
            &"nft add rule inet nym_free_tier confine iifname nymwg ip saddr @garden_v4 ip daddr @allow_v4 accept"
                .to_string()
        ));
        assert!(
            cmds.contains(
                &"nft add rule inet nym_free_tier confine iifname nymwg ip saddr @garden_v4 drop"
                    .to_string()
            )
        );
        let accept = cmds
            .iter()
            .position(|c| c.contains("ip daddr @allow_v4 accept"))
            .unwrap();
        let drop = cmds
            .iter()
            .position(|c| c.ends_with("@garden_v4 drop"))
            .unwrap();
        assert!(accept < drop);
    }

    #[test]
    fn add_and_remove_peer_touch_the_garden_sets() {
        assert_eq!(
            rendered(&garden().add_peer_commands(&ADDRS)),
            vec![
                "nft add element inet nym_free_tier garden_v4 { 10.1.0.5 }".to_string(),
                "nft add element inet nym_free_tier garden_v6 { fd00::5 }".to_string(),
            ]
        );
        assert_eq!(
            rendered(&garden().remove_peer_commands(&ADDRS)),
            vec![
                "nft delete element inet nym_free_tier garden_v4 { 10.1.0.5 }".to_string(),
                "nft delete element inet nym_free_tier garden_v6 { fd00::5 }".to_string(),
            ]
        );
    }
}
