// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The public facade over the free-tier datapath.
//!
//! [`FreeTierEnforcement`] owns the single shared `nft` table, the shared whitelist,
//! and the runner; it composes the rate-limit pool and the walled garden into that one
//! table and exposes the peer-lifecycle transitions the nym-node wiring drives.

use crate::PeerAddrs;
use crate::command::{CommandRunner, CommandSpec};
use crate::error::EnforcementError;
use crate::garden::WalledGarden;
use crate::nft::{self, SetType};
use crate::tc::RateLimitPool;
use crate::{ALLOW_V4, ALLOW_V6, TABLE};
use std::net::IpAddr;
use std::sync::Arc;

/// Free-tier datapath enforcement: the `tc` rate-limit pool and the `nftables` walled
/// garden, both in one shared `inet nym_free_tier` table.
#[derive(Clone)]
pub struct FreeTierEnforcement {
    interface: String,
    pool: RateLimitPool,
    garden: WalledGarden,
    whitelist: Vec<IpAddr>,
    runner: Arc<dyn CommandRunner>,
}

impl FreeTierEnforcement {
    /// `interface` is the WireGuard interface; `pool_bytes_per_sec` the aggregate pool
    /// ceiling; `whitelist` the purchase-endpoint addresses (shared by the pool
    /// exemption and the garden allow).
    pub fn new(
        interface: impl Into<String>,
        pool_bytes_per_sec: u64,
        whitelist: Vec<IpAddr>,
        runner: Arc<dyn CommandRunner>,
    ) -> Self {
        let interface = interface.into();
        FreeTierEnforcement {
            interface: interface.clone(),
            pool: RateLimitPool::new(interface.clone(), pool_bytes_per_sec),
            garden: WalledGarden::new(interface),
            whitelist,
            runner,
        }
    }

    /// The WireGuard interface name.
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Build the whole datapath skeleton: the HTB pool, the shared table with both
    /// subsystems' sets + chains, and the loaded whitelist. Idempotent (torn down
    /// first). The caller reconciles per-peer membership afterwards, BEFORE serving
    /// peers, and MUST NOT call [`Self::teardown`] on shutdown. If `nft` is absent this
    /// fails loudly - that failure is the startup preflight.
    pub fn setup(&self) -> Result<(), EnforcementError> {
        self.run(self.teardown_commands(), true)?;
        self.run(self.setup_commands(), false)
    }

    /// Rebuild the skeleton and reconcile membership from state (the startup path): each
    /// currently-pooled peer into the pool, each currently-gardened peer into the garden.
    pub fn reconcile(
        &self,
        pooled: &[PeerAddrs],
        gardened: &[PeerAddrs],
    ) -> Result<(), EnforcementError> {
        self.setup()?;
        for peer in pooled {
            self.run(self.pool.add_peer_commands(peer), false)?;
        }
        for peer in gardened {
            self.run(self.garden.add_peer_commands(peer), false)?;
        }
        Ok(())
    }

    /// A fresh free peer joins the rate-limit pool (also clearing any stale garden
    /// membership, so the peer ends up in exactly the pool).
    pub fn admit(&self, peer: &PeerAddrs) -> Result<(), EnforcementError> {
        self.run(self.garden.remove_peer_commands(peer), true)?;
        self.run(self.pool.add_peer_commands(peer), false)
    }

    /// Exhaustion transition: leave the pool (full speed) and enter the walled garden.
    pub fn send_to_garden(&self, peer: &PeerAddrs) -> Result<(), EnforcementError> {
        self.run(self.pool.remove_peer_commands(peer), true)?;
        self.run(self.garden.add_peer_commands(peer), false)
    }

    /// Put a peer straight into the garden without pooling it first (renewal tokens).
    pub fn confine(&self, peer: &PeerAddrs) -> Result<(), EnforcementError> {
        self.run(self.garden.add_peer_commands(peer), false)
    }

    /// Release a peer from all free-tier enforcement (paid upgrade): out of both the
    /// pool and the garden, restoring unrestricted full-speed access. Idempotent.
    pub fn release(&self, peer: &PeerAddrs) -> Result<(), EnforcementError> {
        self.run(self.pool.remove_peer_commands(peer), true)?;
        self.run(self.garden.remove_peer_commands(peer), true)
    }

    /// Replace the purchase-endpoint whitelist (external refresh): flush + reload the
    /// shared allow sets, so the pool exemption and the garden allow update together.
    pub fn set_whitelist(&self, whitelist: &[IpAddr]) -> Result<(), EnforcementError> {
        let mut cmds = vec![
            nft::flush_set(TABLE, ALLOW_V4),
            nft::flush_set(TABLE, ALLOW_V6),
        ];
        cmds.extend(whitelist_element_commands(whitelist));
        self.run(cmds, false)
    }

    /// Remove ALL free-tier state (the HTB pool + the whole shared table). For the
    /// explicit cleanup command; NOT called on shutdown.
    pub fn teardown(&self) -> Result<(), EnforcementError> {
        self.run(self.teardown_commands(), true)
    }

    /// The full ordered command sequence built here (`<if>` = the WireGuard interface,
    /// `<rate>` = the pool ceiling in bits/s, `<wl>` = each whitelisted address). Sets
    /// must exist before the chains that reference them, hence this order:
    ///
    ///  1. `tc qdisc add dev <if> root handle 1: htb default 1`
    ///       - root HTB shaper on the WG interface; unclassified packets go to class 1:1.
    ///  2. `tc class add dev <if> parent 1: classid 1:1 htb rate 10gbit ceil 10gbit`
    ///       - the default "unlimited" class (full speed).
    ///  3. `tc class add dev <if> parent 1: classid 1:10 htb rate <rate> ceil <rate>`
    ///       - the shared, rate-limited free-tier pool class.
    ///  4. `nft add table inet nym_free_tier`
    ///       - the single table holding every free-tier rule (teardown = delete it).
    ///  5. `nft add set inet nym_free_tier allow_v4 { type ipv4_addr ; flags interval ; }`
    ///       - the v4 purchase-endpoint whitelist (interval set -> supports CIDRs).
    ///  6. `nft add set inet nym_free_tier allow_v6 { type ipv6_addr ; flags interval ; }`
    ///       - the v6 whitelist.
    ///  7. `nft add element inet nym_free_tier allow_v{4,6} { <wl> }` (one per address)
    ///       - load each whitelisted destination into the shared allow sets.
    ///  8. `nft add set inet nym_free_tier pool_v4 { type ipv4_addr ; }`
    ///       - the set of v4 tunnel IPs currently in the rate-limit pool.
    ///  9. `nft add set inet nym_free_tier pool_v6 { type ipv6_addr ; }`
    ///       - the v6 pool members.
    /// 10. `nft add set inet nym_free_tier garden_v4 { type ipv4_addr ; }`
    ///       - the set of v4 tunnel IPs currently confined to the garden.
    /// 11. `nft add set inet nym_free_tier garden_v6 { type ipv6_addr ; }`
    ///       - the v6 garden members.
    /// 12. `nft add chain inet nym_free_tier classify { type filter hook postrouting priority mangle ; policy accept ; }`
    ///       - the download-classify chain (POSTROUTING runs before tc egress).
    /// 13. `nft add rule inet nym_free_tier classify oifname <if> ip saddr @allow_v4 return`
    ///       - download FROM a whitelisted endpoint skips shaping (stays full speed).
    /// 14. `nft add rule inet nym_free_tier classify oifname <if> ip6 saddr @allow_v6 return`
    ///       - same, v6.
    /// 15. `nft add rule inet nym_free_tier classify oifname <if> ip daddr @pool_v4 meta priority set 1:10`
    ///       - download TO a pooled peer is tagged for the rate-limited class 1:10.
    /// 16. `nft add rule inet nym_free_tier classify oifname <if> ip6 daddr @pool_v6 meta priority set 1:10`
    ///       - same, v6.
    /// 17. `nft add chain inet nym_free_tier confine { type filter hook forward priority -1 ; policy accept ; }`
    ///       - the garden chain (FORWARD, ahead of operator ACCEPTs since it DROPs).
    /// 18. `nft add rule inet nym_free_tier confine iifname <if> ip saddr @garden_v4 ip daddr @allow_v4 accept`
    ///       - a confined peer may reach the whitelist.
    /// 19. `nft add rule inet nym_free_tier confine iifname <if> ip saddr @garden_v4 drop`
    ///       - a confined peer's other outbound traffic is dropped (walled off).
    /// 20. `nft add rule inet nym_free_tier confine iifname <if> ip6 saddr @garden_v6 ip6 daddr @allow_v6 accept`
    ///       - same, v6.
    /// 21. `nft add rule inet nym_free_tier confine iifname <if> ip6 saddr @garden_v6 drop`
    ///       - same, v6.
    fn setup_commands(&self) -> Vec<CommandSpec> {
        let mut cmds = self.pool.htb_commands(); // 1-3: tc HTB shaper + classes
        cmds.push(nft::add_table(TABLE)); // 4: the one shared table
        // 5-7: the shared whitelist sets + their elements (single source of truth,
        // referenced by both the pool exemption and the garden allow)
        cmds.push(nft::add_interval_set(TABLE, ALLOW_V4, SetType::Ipv4));
        cmds.push(nft::add_interval_set(TABLE, ALLOW_V6, SetType::Ipv6));
        cmds.extend(whitelist_element_commands(&self.whitelist));
        // 8-11: per-subsystem peer sets (pool_*, garden_*)...
        cmds.extend(self.pool.set_commands());
        cmds.extend(self.garden.set_commands());
        // 12-21: ...THEN the chains, which reference the sets above
        cmds.extend(self.pool.chain_commands());
        cmds.extend(self.garden.chain_commands());
        cmds
    }

    fn teardown_commands(&self) -> Vec<CommandSpec> {
        let mut cmds = self.pool.htb_teardown_commands();
        cmds.push(nft::delete_table(TABLE));
        cmds
    }

    fn run(&self, cmds: Vec<CommandSpec>, ignore_failure: bool) -> Result<(), EnforcementError> {
        for cmd in cmds {
            self.runner.execute(&cmd, ignore_failure)?;
        }
        Ok(())
    }
}

fn whitelist_element_commands(whitelist: &[IpAddr]) -> Vec<CommandSpec> {
    whitelist
        .iter()
        .map(|addr| match addr {
            IpAddr::V4(a) => nft::add_element(TABLE, ALLOW_V4, &a.to_string()),
            IpAddr::V6(a) => nft::add_element(TABLE, ALLOW_V6, &a.to_string()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::sync::Mutex;

    /// Records the rendered commands it is asked to run, so transitions can be asserted.
    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl CommandRunner for Recorder {
        fn execute(
            &self,
            cmd: &CommandSpec,
            _ignore_failure: bool,
        ) -> Result<(), EnforcementError> {
            self.0.lock().unwrap().push(cmd.rendered());
            Ok(())
        }
    }

    const ADDRS: PeerAddrs = PeerAddrs {
        v4: Ipv4Addr::new(10, 1, 0, 5),
        v6: Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 5),
    };

    fn enforcement(rec: Arc<Recorder>) -> FreeTierEnforcement {
        FreeTierEnforcement::new(
            "nymwg",
            1_250_000,
            vec![IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5))],
            rec,
        )
    }

    fn captured(rec: &Arc<Recorder>) -> Vec<String> {
        rec.0.lock().unwrap().clone()
    }

    #[test]
    fn setup_builds_one_table_with_both_chains_after_all_sets() {
        let rec = Arc::new(Recorder::default());
        enforcement(rec.clone()).setup().unwrap();
        let cmds = captured(&rec);

        assert!(cmds.iter().any(|c| c == "nft add table inet nym_free_tier"));
        assert!(
            cmds.iter()
                .any(|c| c == "nft add element inet nym_free_tier allow_v4 { 203.0.113.5 }")
        );
        assert!(cmds.iter().any(|c| c.contains("classify oifname nymwg")));
        assert!(cmds.iter().any(|c| c.contains("confine iifname nymwg")));
        assert!(
            cmds.iter()
                .any(|c| c.starts_with("tc class add dev nymwg parent 1: classid 1:10"))
        );

        // the table exists before any set; every set exists before any chain rule
        let table = cmds
            .iter()
            .position(|c| c == "nft add table inet nym_free_tier")
            .unwrap();
        let first_set = cmds
            .iter()
            .position(|c| c.starts_with("nft add set"))
            .unwrap();
        let last_set = cmds
            .iter()
            .rposition(|c| c.starts_with("nft add set"))
            .unwrap();
        let first_rule = cmds
            .iter()
            .position(|c| c.starts_with("nft add rule"))
            .unwrap();
        assert!(table < first_set);
        assert!(last_set < first_rule);
    }

    #[test]
    fn admit_clears_garden_then_adds_to_pool() {
        let rec = Arc::new(Recorder::default());
        enforcement(rec.clone()).admit(&ADDRS).unwrap();
        assert_eq!(
            captured(&rec),
            vec![
                "nft delete element inet nym_free_tier garden_v4 { 10.1.0.5 }".to_string(),
                "nft delete element inet nym_free_tier garden_v6 { fd00::5 }".to_string(),
                "nft add element inet nym_free_tier pool_v4 { 10.1.0.5 }".to_string(),
                "nft add element inet nym_free_tier pool_v6 { fd00::5 }".to_string(),
            ]
        );
    }

    #[test]
    fn send_to_garden_leaves_pool_then_confines() {
        let rec = Arc::new(Recorder::default());
        enforcement(rec.clone()).send_to_garden(&ADDRS).unwrap();
        assert_eq!(
            captured(&rec),
            vec![
                "nft delete element inet nym_free_tier pool_v4 { 10.1.0.5 }".to_string(),
                "nft delete element inet nym_free_tier pool_v6 { fd00::5 }".to_string(),
                "nft add element inet nym_free_tier garden_v4 { 10.1.0.5 }".to_string(),
                "nft add element inet nym_free_tier garden_v6 { fd00::5 }".to_string(),
            ]
        );
    }

    #[test]
    fn release_removes_from_both() {
        let rec = Arc::new(Recorder::default());
        enforcement(rec.clone()).release(&ADDRS).unwrap();
        assert_eq!(
            captured(&rec),
            vec![
                "nft delete element inet nym_free_tier pool_v4 { 10.1.0.5 }".to_string(),
                "nft delete element inet nym_free_tier pool_v6 { fd00::5 }".to_string(),
                "nft delete element inet nym_free_tier garden_v4 { 10.1.0.5 }".to_string(),
                "nft delete element inet nym_free_tier garden_v6 { fd00::5 }".to_string(),
            ]
        );
    }
}
