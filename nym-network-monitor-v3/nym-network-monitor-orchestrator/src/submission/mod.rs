// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Submitting each epoch's materialised figures to the performance contract.

use crate::storage::NetworkMonitorStorage;
use crate::submission::plan::{EpochPlan, unsent};
use anyhow::Context;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::{
    PerformanceQueryClient, PerformanceSigningClient,
};
use nym_validator_client::nyxd::nym_performance_contract_common::EpochId;
use std::num::NonZeroUsize;
use tracing::info;

mod plan;

/// Sends each epoch's figures to the performance contract from the orchestrator's own account,
/// resuming from the contract's cursor rather than from any local record of what landed.
pub(crate) struct PerformanceSubmitter<C> {
    client: C,

    storage: NetworkMonitorStorage,

    /// The orchestrator's account, whose cursor the contract keeps.
    address: AccountId,

    /// Most nodes sent in one transaction.
    batch_size: NonZeroUsize,

    /// Coverage below which a test kind is omitted for every node.
    coverage_threshold: f64,
}

impl<C> PerformanceSubmitter<C>
where
    C: PerformanceQueryClient + PerformanceSigningClient + Sync,
{
    pub(crate) fn new(
        client: C,
        storage: NetworkMonitorStorage,
        address: AccountId,
        batch_size: NonZeroUsize,
        coverage_threshold: f64,
    ) -> Self {
        PerformanceSubmitter {
            client,
            storage,
            address,
            batch_size,
            coverage_threshold,
        }
    }

    /// Sends whatever of `epoch`'s plan the contract's cursor has not passed yet, one chunk per
    /// transaction, and returns `Ok` once nothing is left. A failed chunk ends the step, and the next
    /// one resumes from wherever the cursor then is.
    pub(crate) async fn submit(&self, epoch: EpochId) -> anyhow::Result<()> {
        // no entry means the account was retired after startup
        let cursor = self
            .client
            .get_network_monitor(&self.address)
            .await
            .context("failed to read the submission cursor")?
            .info
            .with_context(|| format!("{} is not an authorised network monitor", self.address))?
            .current_submission_metadata;

        let config_scores = self.storage.get_mixnet_epoch_config_scores(epoch).await?;
        let aggregates = self.storage.get_mixnet_epoch_aggregates(epoch).await?;
        let roles = self.storage.get_described_node_roles().await?;

        let plan = EpochPlan::build(&config_scores, aggregates, &roles, self.coverage_threshold);
        // signals (task 4.1): the three omitted flags, from `plan.coverage`

        let planned = plan.nodes.len();
        let unsent = unsent(plan.nodes, epoch, cursor)?;
        info!(
            epoch,
            planned,
            unsent = unsent.len(),
            coverage = ?plan.coverage,
            "submitting to the performance contract"
        );

        for chunk in unsent.chunks(self.batch_size.get()) {
            // signals (task 4.1): count a failed chunk here and a landed one below
            self.client
                .batch_submit_performance(epoch, chunk.to_vec(), None)
                .await
                .with_context(|| format!("failed to submit {} nodes", chunk.len()))?;
        }

        // signal (task 4.1): the last complete epoch
        Ok(())
    }
}
