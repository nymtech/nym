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
use nym_noise::config::{NoiseNetworkView, NoiseNode};
use nym_noise_keys::{NoiseVersion, VersionedNoiseKeyV1};
use nym_validator_client::nyxd::cosmwasm::MsgExecuteContract;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::ExecuteMsg;
use nym_validator_client::nyxd::{AccountId, Any, Msg, Name};
use nyxd_scraper_shared::error::ScraperError;
use nyxd_scraper_shared::{DecodedMessage, MsgModule, ParsedTransactionDetails, parse_msg};
use std::net::{IpAddr, SocketAddr};
use tracing::{debug, error, info};

/// Blockchain message handler for Network Monitor agent authorisation events.
///
/// Watches for `MsgExecuteContract` messages targeting the Network Monitors smart contract
/// and updates the runtime list of authorised agents accordingly.
///
/// # Thread Safety
///
/// Safe to use across threads - updates to `network_monitors` use lock-free ArcSwap internally.
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

    async fn new_agent(&self, address: SocketAddr, bs58_x25519_noise: String, noise_version: u8) {
        debug!("adding new NM agent {address}");

        let Ok(x25519_pubkey) = x25519::PublicKey::from_base58_string(&bs58_x25519_noise) else {
            error!("network monitor agent {address} has announced an invalid noise key - ignoring");
            return;
        };

        let key = VersionedNoiseKeyV1 {
            supported_version: NoiseVersion::from(noise_version),
            x25519_pubkey,
        };

        // add ip to the routing filter
        self.routable_network_monitors.add_known(address.ip());

        // add noise key to the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
        nodes.insert(address.ip(), NoiseNode::new_network_monitor_agent(key));
        self.noise_view.swap_view(update_permit, nodes);
    }

    async fn revoked_agent(&self, address: IpAddr) {
        debug!("revoking NM agent {address}");

        // remove ip from the routing filter
        self.routable_network_monitors.remove_known(address);

        // remove noise key from the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
        nodes.remove(&address);
        self.noise_view.swap_view(update_permit, nodes);
    }

    async fn revoked_all_agents(&self) {
        info!("revoking all NM agents");

        self.routable_network_monitors.reset();

        // remove all noise keys from the known nodes
        let update_permit = self.noise_view.get_update_permit().await;
        let mut nodes = self.noise_view.all_nodes();
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
            | ExecuteMsg::RevokeNetworkMonitorOrchestrator { .. } => {}
        }

        Ok(())
    }
}
