// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Fixtures for performance snapshots as nym-api retains them.

use crate::node_performance::attestation::auxiliary::PerformanceAuxiliary;
use crate::node_performance::attestation::source::PerformanceSnapshotSource;
use crate::support::attested_snapshots::data::{AttestedSnapshot, RetainedEntry};
use nym_contract_attestation::source::mock::mock_digest_snapshot;
use nym_contracts_common::Percent;
use nym_performance_contract_common::{
    EpochId, EpochWeights, KindMedians, LatestPerformance, NodeId, Weights,
};
use nym_test_utils::helpers::dummy_ed25519_keypair;
use std::collections::BTreeMap;
use tendermint::block::Height;
use time::OffsetDateTime;

pub(crate) fn percent(value: u64) -> Percent {
    Percent::from_percentage_value(value).unwrap()
}

/// A committed entry, its score given in percent.
pub(crate) fn committed(
    node_id: NodeId,
    epoch_id: EpochId,
    score: Option<u64>,
) -> LatestPerformance {
    LatestPerformance {
        node_id,
        epoch_id,
        score: score.map(percent),
    }
}

/// A bundle's medians, each given in percent.
pub(crate) fn kind_medians(
    liveness: Option<u64>,
    stress: Option<u64>,
    config: Option<u64>,
) -> KindMedians {
    KindMedians {
        liveness: liveness.map(percent),
        stress: stress.map(percent),
        config: config.map(percent),
    }
}

pub(crate) fn weights() -> EpochWeights {
    EpochWeights {
        effective_from: 1,
        weights: Weights {
            liveness: percent(80),
            stress: percent(20),
        },
    }
}

/// A distinct block time per height.
pub(crate) fn block_time(height: u32) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_700_000_000 + i64::from(height)).unwrap()
}

/// A retained snapshot at `height` holding `records`, with `bonded` bonded and `medians` held
/// beside it.
pub(crate) fn retained_entry(
    height: u32,
    records: Vec<LatestPerformance>,
    bonded: &[NodeId],
    medians: BTreeMap<EpochId, BTreeMap<NodeId, KindMedians>>,
) -> RetainedEntry<PerformanceSnapshotSource> {
    let node_identities = bonded
        .iter()
        .map(|&node_id| (node_id, *dummy_ed25519_keypair(node_id.into()).public_key()))
        .collect();
    let snapshot = AttestedSnapshot::new(
        mock_digest_snapshot(Height::from(height)).signed(&dummy_ed25519_keypair(0)),
        records,
        node_identities,
    );
    let auxiliary = PerformanceAuxiliary::new_for_tests(weights(), block_time(height), medians);
    RetainedEntry::new(snapshot, auxiliary)
}
