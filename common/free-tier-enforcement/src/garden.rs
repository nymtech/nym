// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! `iptables` walled garden for exhausted free-tier peers.
//!
//! Two node-owned `filter`-table chains, in both address families:
//!   * `NYM-GARDEN` - the verdict: `ACCEPT` each whitelisted (purchase) destination,
//!     then `DROP` everything else.
//!   * `NYM-FT-FORWARD` - per-peer membership, jumped from `FORWARD` (scoped to the
//!     WireGuard interface). A confined peer gets a `-s <peerIP> -j NYM-GARDEN` rule
//!     here; a peer with no rule falls through untouched. Keeping membership in the
//!     node's own chain means per-peer changes never modify the operator's `FORWARD`.
//!
//! The node ENSURES this scaffolding itself (`setup`, idempotent) rather than relying
//! on an operator script, so it survives reboots/upgrades. `setup` tears down first,
//! so the caller reconciles per-peer membership (`add_peer`) from state afterwards.
//! Fail-closed is a WIRING concern, not this manager's: the caller reconciles BEFORE
//! serving peers and MUST NOT call [`WalledGarden::teardown`] on shutdown (the kernel
//! keeps the `DROP` rules while the node is down). `teardown` exists for the explicit
//! cleanup command only.

use crate::PeerAddrs;
use crate::command::{CommandRunner, CommandSpec};
use crate::error::EnforcementError;
use crate::iptables::IpTablesFamily;
use std::net::IpAddr;
use std::sync::Arc;

const FILTER: &str = "filter";
/// Verdict chain (allow the whitelist, drop the rest). Spec-named.
const GARDEN_CHAIN: &str = "NYM-GARDEN";
/// Node-owned membership chain jumped from `FORWARD`; holds the per-peer `-s <peer>`
/// jumps into the verdict chain, so per-peer changes never touch operator `FORWARD`.
const FORWARD_CHAIN: &str = "NYM-FT-FORWARD";

/// The walled-garden manager (v1).
#[derive(Clone)]
pub struct WalledGarden {
    interface: String,
    whitelist: Vec<IpAddr>,
    runner: Arc<dyn CommandRunner>,
}

impl WalledGarden {
    /// `interface` is the WireGuard interface peer traffic ingresses on (the garden
    /// jump is scoped to it); `whitelist` are the purchase-endpoint addresses reachable
    /// from inside the garden.
    pub fn new(
        interface: impl Into<String>,
        whitelist: Vec<IpAddr>,
        runner: Arc<dyn CommandRunner>,
    ) -> Self {
        WalledGarden {
            interface: interface.into(),
            whitelist,
            runner,
        }
    }

    /// Ensure the garden scaffolding (verdict + membership chains, the whitelist, and
    /// the `FORWARD` jump) idempotently; existing state is torn down first. The caller
    /// reconciles per-peer membership from state afterwards, and MUST NOT call
    /// [`Self::teardown`] on shutdown (fail-closed / persist-while-down).
    pub fn setup(&self) -> Result<(), EnforcementError> {
        for cmd in self.teardown_commands() {
            self.runner.execute(&cmd, true)?;
        }
        for cmd in self.setup_commands() {
            self.runner.execute(&cmd, false)?;
        }
        Ok(())
    }

    /// Confine a peer to the whitelist (dual-stack).
    pub fn add_peer(&self, addrs: &PeerAddrs) -> Result<(), EnforcementError> {
        for cmd in self.add_peer_commands(addrs) {
            self.runner.execute(&cmd, false)?;
        }
        Ok(())
    }

    /// Release a peer from the garden without disconnecting (off-switch, reused by the
    /// paid upgrade). Idempotent - a peer that was never confined is not an error.
    pub fn remove_peer(&self, addrs: &PeerAddrs) -> Result<(), EnforcementError> {
        for cmd in self.remove_peer_commands(addrs) {
            self.runner.execute(&cmd, true)?;
        }
        Ok(())
    }

    /// Remove ALL garden state (both chains + the `FORWARD` jump + every per-peer
    /// rule). For the explicit cleanup command; NOT called on shutdown.
    pub fn teardown(&self) -> Result<(), EnforcementError> {
        for cmd in self.teardown_commands() {
            self.runner.execute(&cmd, true)?;
        }
        Ok(())
    }

    fn setup_commands(&self) -> Vec<CommandSpec> {
        let mut cmds = Vec::new();
        for iptables in IpTablesFamily::ALL {
            // verdict chain: allow this family's whitelist, then drop the rest
            cmds.push(iptables.new_chain(FILTER, GARDEN_CHAIN));
            for addr in self.whitelist.iter().filter(|a| iptables.matches(a)) {
                let dst = addr.to_string();
                cmds.push(iptables.append(
                    FILTER,
                    GARDEN_CHAIN,
                    &["-d", dst.as_str(), "-j", "ACCEPT"],
                ));
            }
            cmds.push(iptables.append(FILTER, GARDEN_CHAIN, &["-j", "DROP"]));

            // membership chain + the FORWARD jump, inserted at the top: the garden is
            // DROP-based, so it must precede any operator ACCEPT. Scoped to the WG
            // interface so only peer-originated forwarded traffic is garden-checked.
            cmds.push(iptables.new_chain(FILTER, FORWARD_CHAIN));
            cmds.push(iptables.insert(
                FILTER,
                "FORWARD",
                &["-i", self.interface.as_str(), "-j", FORWARD_CHAIN],
            ));
        }
        cmds
    }

    fn teardown_commands(&self) -> Vec<CommandSpec> {
        let mut cmds = Vec::new();
        for iptables in IpTablesFamily::ALL {
            // drop the jump first, then the membership chain (whose per-peer rules
            // reference NYM-GARDEN), then the now-unreferenced verdict chain
            cmds.push(iptables.delete(
                FILTER,
                "FORWARD",
                &["-i", self.interface.as_str(), "-j", FORWARD_CHAIN],
            ));
            cmds.push(iptables.flush_chain(FILTER, FORWARD_CHAIN));
            cmds.push(iptables.delete_chain(FILTER, FORWARD_CHAIN));
            cmds.push(iptables.flush_chain(FILTER, GARDEN_CHAIN));
            cmds.push(iptables.delete_chain(FILTER, GARDEN_CHAIN));
        }
        cmds
    }

    fn add_peer_commands(&self, addrs: &PeerAddrs) -> Vec<CommandSpec> {
        let v4 = addrs.v4.to_string();
        let v6 = addrs.v6.to_string();
        vec![
            IpTablesFamily::V4.append(
                FILTER,
                FORWARD_CHAIN,
                &["-s", v4.as_str(), "-j", GARDEN_CHAIN],
            ),
            IpTablesFamily::V6.append(
                FILTER,
                FORWARD_CHAIN,
                &["-s", v6.as_str(), "-j", GARDEN_CHAIN],
            ),
        ]
    }

    fn remove_peer_commands(&self, addrs: &PeerAddrs) -> Vec<CommandSpec> {
        let v4 = addrs.v4.to_string();
        let v6 = addrs.v6.to_string();
        vec![
            IpTablesFamily::V4.delete(
                FILTER,
                FORWARD_CHAIN,
                &["-s", v4.as_str(), "-j", GARDEN_CHAIN],
            ),
            IpTablesFamily::V6.delete(
                FILTER,
                FORWARD_CHAIN,
                &["-s", v6.as_str(), "-j", GARDEN_CHAIN],
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn garden() -> WalledGarden {
        WalledGarden::new(
            "nymwg",
            vec![
                IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            ],
            Arc::new(crate::command::SystemCommandRunner),
        )
    }

    fn rendered(cmds: &[CommandSpec]) -> Vec<String> {
        cmds.iter().map(CommandSpec::rendered).collect()
    }

    const ADDRS: PeerAddrs = PeerAddrs {
        v4: Ipv4Addr::new(10, 1, 0, 5),
        v6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 5),
    };

    #[test]
    fn setup_builds_verdict_chain_and_forward_jump_per_family() {
        let cmds = rendered(&garden().setup_commands());
        // v4 whitelist ACCEPT then DROP in the verdict chain, v6 whitelist in ip6tables
        assert!(
            cmds.contains(&"iptables -t filter -A NYM-GARDEN -d 203.0.113.5 -j ACCEPT".to_string())
        );
        assert!(cmds.contains(&"iptables -t filter -A NYM-GARDEN -j DROP".to_string()));
        assert!(
            cmds.contains(
                &"ip6tables -t filter -A NYM-GARDEN -d 2001:db8::1 -j ACCEPT".to_string()
            )
        );
        // the FORWARD jump is INSERTED (top) and scoped to the interface
        assert!(
            cmds.contains(&"iptables -t filter -I FORWARD -i nymwg -j NYM-FT-FORWARD".to_string())
        );
        // the DROP terminates the verdict chain (comes after the ACCEPTs)
        let accept = cmds
            .iter()
            .position(|c| c == "iptables -t filter -A NYM-GARDEN -d 203.0.113.5 -j ACCEPT")
            .unwrap();
        let drop = cmds
            .iter()
            .position(|c| c == "iptables -t filter -A NYM-GARDEN -j DROP")
            .unwrap();
        assert!(accept < drop);
    }

    #[test]
    fn add_and_remove_peer_are_symmetric_dual_stack() {
        assert_eq!(
            rendered(&garden().add_peer_commands(&ADDRS)),
            vec![
                "iptables -t filter -A NYM-FT-FORWARD -s 10.1.0.5 -j NYM-GARDEN".to_string(),
                "ip6tables -t filter -A NYM-FT-FORWARD -s fd00::5 -j NYM-GARDEN".to_string(),
            ]
        );
        assert_eq!(
            rendered(&garden().remove_peer_commands(&ADDRS)),
            vec![
                "iptables -t filter -D NYM-FT-FORWARD -s 10.1.0.5 -j NYM-GARDEN".to_string(),
                "ip6tables -t filter -D NYM-FT-FORWARD -s fd00::5 -j NYM-GARDEN".to_string(),
            ]
        );
    }

    #[test]
    fn teardown_drops_jump_before_chains() {
        let cmds = rendered(&garden().teardown_commands());
        assert!(
            cmds.contains(&"iptables -t filter -D FORWARD -i nymwg -j NYM-FT-FORWARD".to_string())
        );
        assert!(cmds.contains(&"iptables -t filter -X NYM-FT-FORWARD".to_string()));
        assert!(cmds.contains(&"iptables -t filter -X NYM-GARDEN".to_string()));
        // the membership chain (which references NYM-GARDEN) is removed before it
        let fwd = cmds
            .iter()
            .position(|c| c == "iptables -t filter -X NYM-FT-FORWARD")
            .unwrap();
        let garden = cmds
            .iter()
            .position(|c| c == "iptables -t filter -X NYM-GARDEN")
            .unwrap();
        assert!(fwd < garden);
    }
}
