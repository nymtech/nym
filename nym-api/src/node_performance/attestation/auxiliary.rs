// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_std::Addr;
use nym_performance_contract_common::{EpochId, EpochWeights, KindMedians, LastSubmission, NodeId};
use nym_validator_client::nyxd::contract_traits::PinnedPerformanceQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::block::Height;
use time::OffsetDateTime;

/// What the annotations need beside a performance snapshot that the digest does not commit,
/// read at the snapshot's height and unproven.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PerformanceAuxiliary {
    /// The weights in force at the snapshot height.
    weights: EpochWeights,

    /// The block time of the snapshot height.
    block_time: OffsetDateTime,

    /// The per-node medians of the newest entry epoch and the one before it.
    medians: BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>,

    /// The last submission as of the snapshot height, `None` before the first one.
    last_submission: Option<SubmissionKey>,
}

/// The fields identifying a submission, held in place of `LastSubmission`, whose measurements
/// skip absent values when serialised and so cannot round-trip through bincode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SubmissionKey {
    block_height: u64,
    epoch_id: EpochId,
    sender: Addr,
    last_node_id: NodeId,
}

impl SubmissionKey {
    fn new(submission: &LastSubmission) -> Option<Self> {
        let data = submission.data.as_ref()?;
        Some(SubmissionKey {
            block_height: submission.block_height,
            epoch_id: data.epoch_id,
            sender: data.sender.clone(),
            last_node_id: data.data.node_id,
        })
    }
}

impl PerformanceAuxiliary {
    /// Every read is pinned at `height`. `newest_epoch` is the largest epoch among the snapshot's
    /// entries, and `previous` the value held for the newest snapshot below `height`.
    pub(crate) async fn read<C>(
        client: &C,
        height: Height,
        block_time: OffsetDateTime,
        newest_epoch: Option<EpochId>,
        previous: Option<PerformanceAuxiliary>,
    ) -> Result<Self, NyxdError>
    where
        C: PinnedPerformanceQueryClient + Sync,
    {
        let last_submission =
            SubmissionKey::new(&client.get_last_submission_at_height(height).await?);
        // instantiation sets the first weights, so a live contract always has some in force
        let weights = client
            .get_current_weights_at_height(height)
            .await?
            .weights
            .ok_or_else(|| {
                NyxdError::extension_query_failure("performance contract", "no weights in force")
            })?;

        let medians = match previous {
            // nothing was submitted since, so the entries and their medians are as they were
            Some(previous) if previous.last_submission == last_submission => previous.medians,
            previous => Self::read_medians(client, height, newest_epoch, previous).await?,
        };

        Ok(PerformanceAuxiliary {
            weights,
            block_time,
            medians,
            last_submission,
        })
    }

    /// The medians of the newest entry epoch and the one before it, which holds the nodes whose
    /// chunk of the newest epoch has not landed yet. An epoch is frozen once the next one begins,
    /// so one older than the recorded submission's is kept as held rather than paged again.
    async fn read_medians<C>(
        client: &C,
        height: Height,
        newest_epoch: Option<EpochId>,
        previous: Option<PerformanceAuxiliary>,
    ) -> Result<BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>, NyxdError>
    where
        C: PinnedPerformanceQueryClient + Sync,
    {
        let Some(newest_epoch) = newest_epoch else {
            return Ok(BTreeMap::new());
        };
        let recorded_epoch = previous
            .as_ref()
            .and_then(|previous| previous.last_submission.as_ref())
            .map(|submission| submission.epoch_id);
        let mut held = previous
            .map(|previous| previous.medians)
            .unwrap_or_default();

        let mut medians = BTreeMap::new();
        for epoch in newest_epoch.saturating_sub(1)..=newest_epoch {
            let frozen = recorded_epoch.is_some_and(|recorded| epoch < recorded);

            let epoch_medians = match held.remove(&epoch) {
                Some(held) if frozen => held,
                _ => client
                    .get_all_epoch_performance_at_height(epoch, height)
                    .await?
                    .into_iter()
                    .map(|performance| (performance.node_id, performance.medians))
                    .collect(),
            };
            medians.insert(epoch, epoch_medians);
        }
        Ok(medians)
    }
}
