// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Real-time blockchain watcher for Network Monitor agents changes.
//!
//! This module processes blockchain transactions involving the Network Monitors smart contract
//! and automatically updates the list of authorised network monitor agents whenever it is invoked.
//!
//! # Authorisation Flow
//!
//! 1. Network Monitor orchestrator submits `AuthoriseNetworkMonitor { address }` to the contract
//! 2. Transaction is committed to Nyx blockchain
//! 3. This module receives the message via `MsgModule::handle_msg()`
//! 4. The IP address is added to `DeclaredNetworkMonitors` (lock-free via ArcSwap)
//! 5. Future packets from that IP can bypass replay protection until revoked
//!
//! # Security
//!
//! Only transactions executed against the configured Network Monitors contract address are
//! processed.

use crate::node::routing_filter::network_filter::RoutableNetworkMonitors;
use async_trait::async_trait;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{NetworkMonitorAgentNode, NoiseNetworkView, NoiseNode};
use nym_noise_keys::{NoiseVersion, VersionedNoiseKeyV1};
use nym_validator_client::nyxd::cosmwasm::MsgExecuteContract;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::ExecuteMsg;
use nym_validator_client::nyxd::{AccountId, Any, Msg, Name};
use nyxd_scraper_shared::error::ScraperError;
use nyxd_scraper_shared::{DecodedMessage, MsgModule, ParsedTransactionDetails, parse_msg};
use std::net::SocketAddr;
use tracing::{debug, error, info, warn};

/// Blockchain message handler for Network Monitor agent authorisation events.
///
/// Watches for `MsgExecuteContract` messages targeting the Network Monitors smart contract
/// and updates the runtime list of authorised agents accordingly.
pub(crate) struct NetworkMonitorAgentsModule {
    /// The on-chain address of the Network Monitors smart contract.
    /// Only messages to this contract are processed.
    pub(crate) contract_address: AccountId,

    /// Shared handle to the runtime list of authorised network monitor IPs.
    /// Updates are immediately visible to all packet processing threads.
    pub(crate) routable_network_monitors: RoutableNetworkMonitors,

    /// Shared handle to the runtime list of noise keys of all network nodes
    pub(crate) noise_view: NoiseNetworkView,
}

impl NetworkMonitorAgentsModule {
    pub(crate) fn new(
        contract_address: AccountId,
        routable_network_monitors: RoutableNetworkMonitors,
        noise_view: NoiseNetworkView,
    ) -> Self {
        Self {
            contract_address,
            routable_network_monitors,
            noise_view,
        }
    }

    /// Register a newly authorised NM agent in both the routing filter and the noise key map.
    async fn new_agent(
        &mut self,
        address: SocketAddr,
        bs58_x25519_noise: String,
        noise_version: u8,
    ) {
        debug!("adding new NM agent {address}");

        let Ok(x25519_pubkey) = x25519::PublicKey::from_base58_string(&bs58_x25519_noise) else {
            error!("network monitor agent {address} has announced an invalid noise key - ignoring");
            return;
        };

        let key = VersionedNoiseKeyV1 {
            supported_version: NoiseVersion::from(noise_version),
            x25519_pubkey,
        };

        // add ip to the routing filter (it's a no-op if it already exists)
        self.routable_network_monitors.add_known(address.ip());

        // add noise key to the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
        // canonicalise so lookups via supports_noise (which canonicalises) always match
        let ip = address.ip().to_canonical();
        let port = address.port();

        match nodes.get_mut(&ip) {
            None => {
                nodes.insert(ip, NoiseNode::new_agent(address, key));
            }
            Some(existing_entry) => match existing_entry {
                NoiseNode::NymNode { .. } => {
                    error!(
                        "the authorised agent runs on the same host as a known nym-node! ignoring"
                    );
                }
                NoiseNode::NetworkMonitorAgent { nodes } => {
                    if let Some(existing) = nodes.iter_mut().find(|n| n.port == address.port()) {
                        existing.key = key;
                    } else {
                        nodes.push(NetworkMonitorAgentNode { port, key });
                    }
                }
            },
        }

        self.noise_view.swap_view(update_permit, nodes);
    }

    async fn revoked_agent(&mut self, address: SocketAddr) {
        debug!("revoking NM agent {address}");

        // canonicalise to match the stored representation
        let ip = address.ip().to_canonical();

        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();

        let mut final_agent = false;
        match nodes.get_mut(&ip) {
            None => {
                warn!("attempted to revoke a non-existent agent at {address}");
                return;
            }
            Some(node) => match node {
                NoiseNode::NymNode { .. } => {
                    error!(
                        "the revoked agent runs on the same host as a known nym-node! ignoring the revocation"
                    );
                    return;
                }
                NoiseNode::NetworkMonitorAgent { nodes } => {
                    nodes.retain(|agent| agent.port != address.port());
                    if nodes.is_empty() {
                        final_agent = true;
                    }
                }
            },
        }

        if final_agent {
            nodes.remove(&ip);
            self.routable_network_monitors.remove_known(ip);
        }
        self.noise_view.swap_view(update_permit, nodes);
    }

    async fn revoked_all_agents(&mut self) {
        info!("revoking all NM agents");

        self.routable_network_monitors.reset();

        // remove all noise keys from the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();

        // Only remove NM agent entries; nym-node entries must be preserved because they are
        // managed by a completely separate code path (the nym-api topology refresher) and
        // would not be restored until the next full topology refresh cycle.
        nodes.retain(|_, node| node.is_nym_node());
        self.noise_view.swap_view(update_permit, nodes);
    }
}

#[async_trait]
impl MsgModule for NetworkMonitorAgentsModule {
    // we're only interested in contract messages
    fn type_url(&self) -> String {
        <MsgExecuteContract as Msg>::Proto::type_url()
    }

    async fn handle_msg(
        &mut self,
        _: usize,
        msg: &Any,
        _: &DecodedMessage,
        tx: &ParsedTransactionDetails,
    ) -> Result<(), ScraperError> {
        // don't process failed transactions
        if !tx.tx_result.code.is_ok() {
            return Ok(());
        }

        // propagate error as this is a critical failure indicating our code is incompatible with
        // the current CometBFT schema so parsing can't proceed
        let execute_msg: MsgExecuteContract = parse_msg(msg)?;

        // not the contract we're interested in
        if execute_msg.contract != self.contract_address {
            return Ok(());
        }

        let exec_msg: ExecuteMsg = match serde_json::from_slice(&execute_msg.msg) {
            Ok(msg) => msg,
            Err(err) => {
                // do NOT propagate error. this just means the contract might have updated.
                // further block processing should continue
                error!(
                    "failed to parse out network monitors contract ExecuteMsg - has the contact schema been updated? error was: {err}"
                );
                return Ok(());
            }
        };

        match exec_msg {
            ExecuteMsg::AuthoriseNetworkMonitor {
                mixnet_address,
                bs58_x25519_noise,
                noise_version,
            } => {
                self.new_agent(mixnet_address, bs58_x25519_noise, noise_version)
                    .await
            }
            ExecuteMsg::RevokeNetworkMonitor { address } => self.revoked_agent(address).await,
            ExecuteMsg::RevokeAllNetworkMonitors => self.revoked_all_agents().await,

            // we're not interested in those messages
            ExecuteMsg::UpdateAdmin { .. }
            | ExecuteMsg::AuthoriseNetworkMonitorOrchestrator { .. }
            | ExecuteMsg::RevokeNetworkMonitorOrchestrator { .. }
            | ExecuteMsg::UpdateOrchestratorIdentityKey { .. } => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::x25519;
    use nym_test_utils::helpers::deterministic_rng;
    use std::net::{IpAddr, Ipv4Addr};

    fn module() -> NetworkMonitorAgentsModule {
        NetworkMonitorAgentsModule::new(
            "n1pefc2utwpy5w78p2kqdsfmpjxfwmn9d39k5mqa".parse().unwrap(),
            RoutableNetworkMonitors::default(),
            NoiseNetworkView::new_empty(),
        )
    }

    // Regression: an agent registered via blockchain events must end up keyed in the noise map
    // under the **canonical** IP form, so the responder's `supports_noise` (which canonicalises
    // on lookup) finds it regardless of whether the inbound socket presents plain IPv4 or the
    // v4-mapped IPv6 form. Before the fix, `new_agent` inserted `address.ip()` raw, leaving the
    // map keyed on a non-canonical IPv4-mapped IPv6 address whenever the contract submission used
    // that form, while the routing filter (which canonicalises on both sides) accepted the
    // packet — producing the "can't speak Noise yet, falling back to TCP" warning.
    #[tokio::test]
    async fn new_agent_stores_under_canonical_ip() {
        let mut module = module();
        let pubkey = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut deterministic_rng()));
        let bs58 = pubkey.to_base58_string();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let v6_mapped = IpAddr::V6(Ipv4Addr::new(1, 2, 3, 4).to_ipv6_mapped());

        // register agent using v4-mapped IPv6 form (the form that triggered the bug)
        module
            .new_agent(SocketAddr::new(v6_mapped, 39322), bs58, 1)
            .await;

        let stored = module.noise_view.all_nodes();
        // the stored key must be canonical so canonical-form lookups succeed
        assert!(
            stored.contains_key(&v4),
            "noise map must contain the canonical IPv4 key, got: {:?}",
            stored.keys().collect::<Vec<_>>()
        );
    }

    // Counterpart: same invariant when the contract submission already used plain IPv4 — the
    // map should still be keyed on the canonical form (which for plain IPv4 is itself).
    #[tokio::test]
    async fn new_agent_stores_under_canonical_ip_for_plain_v4_input() {
        let mut module = module();
        let pubkey = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut deterministic_rng()));
        let bs58 = pubkey.to_base58_string();

        let v4 = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

        module.new_agent(SocketAddr::new(v4, 39322), bs58, 1).await;

        let stored = module.noise_view.all_nodes();
        assert!(stored.contains_key(&v4));
    }
}
