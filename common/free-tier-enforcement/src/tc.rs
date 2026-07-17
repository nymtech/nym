// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Shared `tc` HTB rate-limit pool for free-tier peers.
//!
//! v1 shapes the EGRESS (download, node -> peer) direction: an HTB pool class on the
//! WireGuard interface, with free peers steered into it by `iptables` / `ip6tables`
//! mangle `CLASSIFY` rules keyed on the peer's tunnel IP (dual-stack). Unclassified
//! traffic stays in the unlimited default class, so removing a peer's rule is the
//! off-switch (task 4.3). Purchase-endpoint destinations are exempted (full speed,
//! task 4.5) ahead of the per-peer rules.
//!
//! Ingress (upload) shaping is NOT here yet: netfilter POSTROUTING runs before `tc`
//! egress (so `CLASSIFY` works for download), but `tc` ingress runs BEFORE netfilter
//! PREROUTING, so the upload path needs an IFB device + `tc` u32 filters rather than
//! `iptables CLASSIFY`. That lands with the dual-direction harness validation (7.2).

use crate::PeerAddrs;
use crate::command::{CommandRunner, CommandSpec};
use crate::error::EnforcementError;
use crate::iptables::IpTablesFamily;
use std::net::IpAddr;

const MANGLE: &str = "mangle";

/// The node-owned mangle chain that classifies free peers' download traffic; jumped
/// from POSTROUTING for the WireGuard interface, never touching operator rules.
const DOWNLOAD_CHAIN: &str = "NYM-FT-DL";
const POOL_CLASSID: &str = "1:10";
const UNLIMITED_CLASSID: &str = "1:1";
/// Effectively-unlimited ceiling for the default class (unclassified = full speed).
const UNLIMITED_RATE: &str = "10gbit";

/// The shared `tc` rate-limit pool manager (download direction, v1).
#[derive(Clone)]
pub struct RateLimitPool {
    interface: String,
    pool_rate_bits_per_sec: u64,
    whitelist: Vec<IpAddr>,
    runner: Box<dyn CommandRunner>,
}

impl RateLimitPool {
    /// `pool_bytes_per_sec` is the aggregate free-pool ceiling; `allowlist` holds the
    /// purchase-endpoint addresses exempted from shaping (full speed).
    pub fn new(
        interface: impl Into<String>,
        pool_bytes_per_sec: u64,
        whitelist: Vec<IpAddr>,
        runner: Box<dyn CommandRunner>,
    ) -> Self {
        RateLimitPool {
            interface: interface.into(),
            pool_rate_bits_per_sec: pool_bytes_per_sec.saturating_mul(8),
            whitelist,
            runner,
        }
    }

    /// One-time HTB structure + the mangle classify chain and its exemptions. Safe to
    /// re-run (idempotent): existing state is torn down first, so it doubles as the
    /// startup reconcile of the pool skeleton (peers are re-added by [`Self::add_peer`]).
    pub fn setup(&self) -> Result<(), EnforcementError> {
        for cmd in self.teardown_commands() {
            self.runner.execute(&cmd, true)?;
        }
        for cmd in self.setup_commands() {
            self.runner.execute(&cmd, false)?;
        }
        Ok(())
    }

    /// Steer a peer's download traffic into the shared pool (dual-stack).
    pub fn add_peer(&self, addrs: &PeerAddrs) -> Result<(), EnforcementError> {
        for cmd in self.add_peer_commands(addrs) {
            self.runner.execute(&cmd, false)?;
        }
        Ok(())
    }

    /// Off-switch (task 4.3): remove the peer's classify rule so it falls back to the
    /// unlimited default class, without disconnecting. Idempotent - a missing rule (a
    /// peer that was never pooled, e.g. on the upgrade path) is not an error.
    pub fn remove_peer(&self, addrs: &PeerAddrs) -> Result<(), EnforcementError> {
        for cmd in self.remove_peer_commands(addrs) {
            self.runner.execute(&cmd, true)?;
        }
        Ok(())
    }

    fn pool_rate(&self) -> String {
        format!("{}bit", self.pool_rate_bits_per_sec)
    }

    fn setup_commands(&self) -> Vec<CommandSpec> {
        let dev = self.interface.as_str();
        let pool_rate = self.pool_rate();
        let mut cmds = vec![
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
        ];

        for iptables in IpTablesFamily::ALL {
            cmds.push(iptables.new_chain(MANGLE, DOWNLOAD_CHAIN));

            // whitelist (e.g. purchase) exemption: download FROM an allowlisted endpoint is full
            // speed - RETURN ahead of the per-peer CLASSIFY rules appended later.
            for addr in self.whitelist.iter().filter(|a| iptables.matches(a)) {
                let src = addr.to_string();
                cmds.push(iptables.append(
                    MANGLE,
                    DOWNLOAD_CHAIN,
                    &["-s", src.as_str(), "-j", "RETURN"],
                ));
            }
            cmds.push(iptables.append(MANGLE, "POSTROUTING", &["-o", dev, "-j", DOWNLOAD_CHAIN]));
        }
        cmds
    }

    fn teardown_commands(&self) -> Vec<CommandSpec> {
        let dev = self.interface.as_str();
        let mut cmds = vec![CommandSpec::new("tc", ["qdisc", "del", "dev", dev, "root"])];
        for iptables in IpTablesFamily::ALL {
            cmds.push(iptables.delete(MANGLE, "POSTROUTING", &["-o", dev, "-j", DOWNLOAD_CHAIN]));
            cmds.push(iptables.flush_chain(MANGLE, DOWNLOAD_CHAIN));
            cmds.push(iptables.delete_chain(MANGLE, DOWNLOAD_CHAIN));
        }
        cmds
    }

    fn add_peer_commands(&self, addrs: &PeerAddrs) -> Vec<CommandSpec> {
        let v4 = addrs.v4.to_string();
        let v6 = addrs.v6.to_string();

        vec![
            IpTablesFamily::V4.append(
                MANGLE,
                DOWNLOAD_CHAIN,
                &[
                    "-d",
                    v4.as_str(),
                    "-j",
                    "CLASSIFY",
                    "--set-class",
                    POOL_CLASSID,
                ],
            ),
            IpTablesFamily::V6.append(
                MANGLE,
                DOWNLOAD_CHAIN,
                &[
                    "-d",
                    v6.as_str(),
                    "-j",
                    "CLASSIFY",
                    "--set-class",
                    POOL_CLASSID,
                ],
            ),
        ]
    }

    fn remove_peer_commands(&self, addrs: &PeerAddrs) -> Vec<CommandSpec> {
        let v4 = addrs.v4.to_string();
        let v6 = addrs.v6.to_string();
        vec![
            IpTablesFamily::V4.delete(
                MANGLE,
                DOWNLOAD_CHAIN,
                &[
                    "-d",
                    v4.as_str(),
                    "-j",
                    "CLASSIFY",
                    "--set-class",
                    POOL_CLASSID,
                ],
            ),
            IpTablesFamily::V6.delete(
                MANGLE,
                DOWNLOAD_CHAIN,
                &[
                    "-d",
                    v6.as_str(),
                    "-j",
                    "CLASSIFY",
                    "--set-class",
                    POOL_CLASSID,
                ],
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn pool() -> RateLimitPool {
        RateLimitPool::new(
            "nymwg",
            1_250_000, // 1.25 MB/s -> 10_000_000 bit/s
            vec![
                IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            ],
            Box::new(crate::command::SystemCommandRunner),
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
    fn setup_builds_htb_pool_at_the_configured_rate() {
        let cmds = rendered(&pool().setup_commands());
        assert!(cmds.contains(&"tc qdisc add dev nymwg root handle 1: htb default 1".to_string()));
        assert!(cmds.contains(
            &"tc class add dev nymwg parent 1: classid 1:10 htb rate 10000000bit ceil 10000000bit"
                .to_string()
        ));
    }

    #[test]
    fn setup_exempts_purchase_endpoints_per_family_before_the_jump() {
        let cmds = rendered(&pool().setup_commands());
        // v4 endpoint exempted via iptables, v6 endpoint via ip6tables, each RETURN
        assert!(
            cmds.contains(&"iptables -t mangle -A NYM-FT-DL -s 203.0.113.5 -j RETURN".to_string())
        );
        assert!(
            cmds.contains(&"ip6tables -t mangle -A NYM-FT-DL -s 2001:db8::1 -j RETURN".to_string())
        );
        // the RETURN exemption precedes the POSTROUTING jump for its family
        let v4_return = cmds
            .iter()
            .position(|c| c == "iptables -t mangle -A NYM-FT-DL -s 203.0.113.5 -j RETURN")
            .unwrap();
        let v4_jump = cmds
            .iter()
            .position(|c| c == "iptables -t mangle -A POSTROUTING -o nymwg -j NYM-FT-DL")
            .unwrap();
        assert!(v4_return < v4_jump);
    }

    #[test]
    fn add_peer_classifies_both_families_into_the_pool() {
        assert_eq!(
            rendered(&pool().add_peer_commands(&ADDRS)),
            vec![
                "iptables -t mangle -A NYM-FT-DL -d 10.1.0.5 -j CLASSIFY --set-class 1:10"
                    .to_string(),
                "ip6tables -t mangle -A NYM-FT-DL -d fd00::5 -j CLASSIFY --set-class 1:10"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn remove_peer_deletes_the_same_rules_it_added() {
        assert_eq!(
            rendered(&pool().remove_peer_commands(&ADDRS)),
            vec![
                "iptables -t mangle -D NYM-FT-DL -d 10.1.0.5 -j CLASSIFY --set-class 1:10"
                    .to_string(),
                "ip6tables -t mangle -D NYM-FT-DL -d fd00::5 -j CLASSIFY --set-class 1:10"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn teardown_removes_jump_then_chain_for_both_families() {
        let cmds = rendered(&pool().teardown_commands());
        assert!(cmds.contains(&"tc qdisc del dev nymwg root".to_string()));
        assert!(
            cmds.contains(&"iptables -t mangle -D POSTROUTING -o nymwg -j NYM-FT-DL".to_string())
        );
        assert!(cmds.contains(&"ip6tables -t mangle -X NYM-FT-DL".to_string()));
    }
}
