// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Test-only fixtures shared across the verification and routing suites.

use nym_crypto::asymmetric::ed25519;
use nym_performance_contract_common::{EpochId, LatestPerformance, NodeId};
use nym_test_utils::helpers::dummy_ed25519_keypair;
use std::collections::BTreeMap;

/// A committed entry, its score parsed from a decimal string.
pub(crate) fn entry(node_id: NodeId, epoch_id: EpochId, score: Option<&str>) -> LatestPerformance {
    LatestPerformance {
        node_id,
        epoch_id,
        score: score.map(|score| score.parse().unwrap()),
    }
}

/// The identity map of `node_ids` bonded, each with a deterministic key.
pub(crate) fn bonded(node_ids: &[NodeId]) -> BTreeMap<NodeId, ed25519::PublicKey> {
    node_ids
        .iter()
        .map(|&node_id| {
            let key = *dummy_ed25519_keypair(node_id.into()).public_key();
            (node_id, key)
        })
        .collect()
}
