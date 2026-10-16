// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! A local copy of the performance contract's per-epoch medians, enough to resolve every node's
//! figure at the current epoch with the contract's own rule.

use nym_contracts_common::Percent;
use nym_mixnet_contract_common::{EpochId, NodeId};
use nym_validator_client::nyxd::contract_traits::performance_query_client::{
    EpochWeights, KindMedians, LastSubmission,
};
use nym_validator_client::nyxd::contract_traits::PerformanceQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockReadGuard};

pub(crate) mod refresher;

/// Shared handle to the window, holding nothing until its first successful load.
#[derive(Clone, Default)]
pub(crate) struct PerformanceContractWindow {
    inner: Arc<RwLock<Option<WindowData>>>,
}

impl PerformanceContractWindow {
    /// The window's data, unless it has never loaded.
    pub(crate) async fn read(&self) -> Option<RwLockReadGuard<'_, WindowData>> {
        RwLockReadGuard::try_map(self.inner.read().await, Option::as_ref).ok()
    }

    /// One refresh with `current_epoch` as X: a full load if the window never loaded, else a
    /// re-page of whatever changed since the last submission it recorded.
    pub(crate) async fn poll<C>(&self, client: &C, current_epoch: EpochId) -> Result<(), NyxdError>
    where
        C: PerformanceQueryClient + Sync,
    {
        let _ = (client, current_epoch);
        todo!("task 3.2")
    }
}

/// The contract's per-node medians for `[R - 24, R]` and the current epoch X, with the weights at X.
pub(crate) struct WindowData {
    /// An epoch read with no nodes is not held, so it never counts as R.
    epochs: BTreeMap<EpochId, HashMap<NodeId, KindMedians>>,

    current_epoch: EpochId,

    weights: Option<EpochWeights>,

    /// The newest submission seen, read before the pages it triggered; a change means its epoch
    /// has to be re-paged.
    last_submission: LastSubmission,

    refreshed_at: OffsetDateTime,
}

impl WindowData {
    pub(crate) fn current_epoch(&self) -> EpochId {
        self.current_epoch
    }

    pub(crate) fn refreshed_at(&self) -> OffsetDateTime {
        self.refreshed_at
    }

    /// Whether the contract has ever received a submission.
    pub(crate) fn has_submission(&self) -> bool {
        self.last_submission.data.is_some()
    }

    /// Whether anything was submitted for the current epoch, i.e. R is X.
    pub(crate) fn current_epoch_has_data(&self) -> bool {
        self.epochs.contains_key(&self.current_epoch)
    }

    /// The node's figure at the current epoch, by the contract's own resolution rule.
    pub(crate) fn figure(&self, node_id: NodeId) -> Option<ContractFigure> {
        let _ = (node_id, &self.epochs, &self.weights);
        todo!("task 3.1")
    }
}

/// A node's resolved contract figure at the window's current epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContractFigure {
    pub(crate) source_epoch: EpochId,

    pub(crate) medians: KindMedians,

    /// `None` when the source cannot be scored (no weighted kind measured, or no config).
    pub(crate) score: Option<Percent>,
}
