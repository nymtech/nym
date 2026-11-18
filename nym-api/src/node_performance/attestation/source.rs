// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_performance::attestation::auxiliary::PerformanceAuxiliary;
use crate::support::attested_snapshots::source::{AttestedSnapshotSource, VerifiedSet};
use crate::support::chain_anchor::{SharedChainAnchor, SharedContractAnchor};
use anyhow::Context;
use async_trait::async_trait;
use nym_contract_anchor::error::VerificationError;
use nym_performance_client::client::PerformanceClient;
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::LatestPerformance;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{AccountId, TendermintRpcClient};
use nym_validator_client::QueryHttpRpcNyxdClient;
use tendermint::block::Height;
use time::OffsetDateTime;

/// Every committed performance entry at a height, verified against the digest proven there.
pub(crate) struct PerformanceSnapshotSource {
    contract_address: AccountId,

    performance_client: PerformanceClient<SharedContractAnchor, QueryHttpRpcNyxdClient>,
}

impl PerformanceSnapshotSource {
    pub(crate) fn new(chain_anchor: &SharedChainAnchor) -> anyhow::Result<Self> {
        let query_client = chain_anchor.query_client().clone_query_client();
        let contract_address = query_client
            .performance_contract_address()
            .context("failed to get performance contract address")?
            .clone();
        let anchor = chain_anchor.contract_anchor(
            contract_address.clone(),
            storage_keys::DIGEST_STATE.as_bytes().to_vec(),
        );

        Ok(PerformanceSnapshotSource {
            contract_address,
            performance_client: PerformanceClient::new(anchor, query_client),
        })
    }
}

#[async_trait]
impl AttestedSnapshotSource for PerformanceSnapshotSource {
    type Record = LatestPerformance;
    type Auxiliary = PerformanceAuxiliary;

    const NAME: &'static str = "performance";

    fn contract_address(&self) -> &AccountId {
        &self.contract_address
    }

    async fn verified_set(
        &self,
        height: Height,
        previous: Option<PerformanceAuxiliary>,
    ) -> Result<VerifiedSet<Self>, VerificationError> {
        let performance = self.performance_client.verified_performance(height).await?;

        let client = self.performance_client.client();
        let block_time = client
            .header(height)
            .await
            .map_err(NyxdError::from)?
            .header
            .time;
        let newest_epoch = performance
            .entries
            .values()
            .map(|entry| entry.epoch_id)
            .max();
        let auxiliary = PerformanceAuxiliary::read(
            client,
            height,
            OffsetDateTime::from(block_time),
            newest_epoch,
            previous,
        )
        .await?;

        Ok(VerifiedSet {
            accumulator: performance.accumulator,
            // keyed by node, so in ascending node order, which the per-node lookup's binary search
            // relies on
            records: performance.entries.into_values().collect(),
            node_identities: performance.node_identities,
            auxiliary,
        })
    }
}
