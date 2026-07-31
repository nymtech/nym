// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::directory_publisher::DEFAULT_MINIMUM_ON_CHAIN_BALANCE_AMOUNT;
use crate::node::nyx_client::NyxClient;
use async_trait::async_trait;
use nym_topology::NodeId;
use nym_validator_client::nyxd::contract_traits::{
    DirectoryQueryClient, DirectorySigningClient, MixnetQueryClient,
};
use nym_validator_client::nyxd::module_traits::feegrant::query::FeegrantQueryClient;
use nym_validator_client::rpc::TendermintRpcClientExt;
use std::ops::Deref;
use tracing::debug;

/// The chain operations the [`DirectoryPublisher`](super::DirectoryPublisher) needs, expressed
/// at the publisher's semantic level rather than as raw contract queries. This is the seam that
/// lets the publisher be unit-tested against an in-memory fake instead of a live chain: the
/// production implementation ([`NyxClient`]) performs the real queries plus trivial mapping,
/// while a test double returns canned values. Kept deliberately small - only what the publisher
/// actually calls.
#[async_trait]
pub(crate) trait DirectoryChainClient: Send + Sync {
    /// Whether a directory contract address is configured for this network (else the publisher
    /// must not run).
    async fn directory_contract_configured(&self) -> Result<bool, NymNodeError>;

    /// This node's `node_id` iff `identity` is bonded and *not* unbonding, else `None` (the
    /// contract rejects writes in both cases). `Err` only when the lookup itself failed.
    async fn node_id(&self, identity: String) -> Result<Option<NodeId>, NymNodeError>;

    /// Whether the node account's on-chain balance is sufficient to pay tx fees.
    async fn has_sufficient_on_chain_balance(&self) -> Result<bool, NymNodeError>;

    /// Whether the relayer account has any feegrant allowance.
    async fn has_feegrant(&self) -> Result<bool, NymNodeError>;

    /// Every entry currently published by this node, as `(label, data)` pairs.
    async fn node_entries(&self, node_id: NodeId) -> Result<Vec<(String, Vec<u8>)>, NymNodeError>;

    /// The next sequence the contract expects this node to sign with.
    async fn next_sequence(&self, node_id: NodeId) -> Result<u64, NymNodeError>;

    /// The contract's current label whitelist, as raw label strings.
    async fn allowed_labels(&self) -> Result<Vec<String>, NymNodeError>;

    /// Relay a single `set_node_entry`; the caller owns sequencing and retry.
    async fn set_entry(
        &self,
        node_id: NodeId,
        label: String,
        data: Vec<u8>,
        sequence: u64,
        signature: Vec<u8>,
    ) -> Result<(), NymNodeError>;

    /// Relay a single `delete_node_entry`; the caller owns sequencing and retry.
    async fn delete_entry(
        &self,
        node_id: NodeId,
        label: String,
        sequence: u64,
        signature: Vec<u8>,
    ) -> Result<(), NymNodeError>;
}

#[async_trait]
impl DirectoryChainClient for NyxClient {
    async fn directory_contract_configured(&self) -> Result<bool, NymNodeError> {
        Ok(self
            .get_nym_contracts()
            .await
            .directory_contract_address
            .is_some())
    }

    async fn node_id(&self, identity: String) -> Result<Option<NodeId>, NymNodeError> {
        let details = self
            .read()
            .await
            .get_nymnode_details_by_identity(identity)
            .await?
            .details;

        Ok(match details {
            Some(node) if !node.is_unbonding() => Some(node.node_id()),
            _ => None,
        })
    }

    async fn has_sufficient_on_chain_balance(&self) -> Result<bool, NymNodeError> {
        let client = self.read().await;
        let address = client.address();
        let denom = &client.current_config().chain_details().mix_denom.base;
        let balance = client.get_balance(&address, denom.to_string()).await?;

        match balance {
            Some(balance) if balance.amount >= DEFAULT_MINIMUM_ON_CHAIN_BALANCE_AMOUNT => {
                debug!("relayer balance {balance} is sufficient for directory writes");
                Ok(true)
            }
            Some(balance) => {
                debug!(
                    "relayer balance {balance} is insufficient for directory writes; checking the feegrant..."
                );
                Ok(false)
            }
            None => {
                debug!("relayer has no on-chain balance; checking the feegrant...");
                Ok(false)
            }
        }
    }

    async fn has_feegrant(&self) -> Result<bool, NymNodeError> {
        let client = self.read().await;
        let address = client.address();
        let allowances = client.allowances(address, None).await?;
        Ok(!allowances.allowances.is_empty())
    }

    async fn node_entries(&self, node_id: NodeId) -> Result<Vec<(String, Vec<u8>)>, NymNodeError> {
        let on_chain = self.read().await.get_node_entries(node_id).await?;
        Ok(on_chain
            .entries
            .into_iter()
            .map(|labelled| (labelled.label, labelled.entry.data.as_slice().to_vec()))
            .collect())
    }

    async fn next_sequence(&self, node_id: NodeId) -> Result<u64, NymNodeError> {
        let client = self.read().await;
        Ok(DirectoryQueryClient::get_sequence(client.deref(), node_id)
            .await?
            .next_sequence)
    }

    async fn allowed_labels(&self) -> Result<Vec<String>, NymNodeError> {
        let allowed = self.read().await.get_allowed_labels().await?;
        Ok(allowed
            .labels
            .into_iter()
            .map(|entry| entry.label)
            .collect())
    }

    async fn set_entry(
        &self,
        node_id: NodeId,
        label: String,
        data: Vec<u8>,
        sequence: u64,
        signature: Vec<u8>,
    ) -> Result<(), NymNodeError> {
        self.write()
            .await
            .set_node_entry(
                node_id,
                label,
                data.into(),
                sequence,
                signature.into(),
                None,
            )
            .await?;
        Ok(())
    }

    async fn delete_entry(
        &self,
        node_id: NodeId,
        label: String,
        sequence: u64,
        signature: Vec<u8>,
    ) -> Result<(), NymNodeError> {
        self.write()
            .await
            .delete_node_entry(node_id, label, sequence, signature.into(), None)
            .await?;
        Ok(())
    }
}
