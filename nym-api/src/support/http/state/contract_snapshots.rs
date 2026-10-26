// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::directory::attestation::source::DirectorySnapshotSource;
use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::support::attested_snapshots::state::AttestedSnapshotsState;

/// The verified snapshots of every contract this nym-api attests.
#[derive(Clone)]
pub(crate) struct ContractSnapshots {
    pub(crate) directory: AttestedSnapshotsState<DirectorySnapshotSource>,

    pub(crate) performance: AttestedSnapshotsState<PerformanceSnapshotSource>,
}
