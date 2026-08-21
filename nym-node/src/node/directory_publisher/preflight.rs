// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use nym_topology::NodeId;
use tracing::{error, warn};

/// The outcome of a startup / back-off preflight check.
pub(crate) enum Preflight {
    /// Bonded, funded, and `node_id` resolved - the publisher may write.
    Ready(NodeId),

    /// Not an active (bonded, non-unbonding) node yet.
    NotBonded,

    /// Bonded, but the relayer account cannot currently fund writes.
    NotFundable,
}

/// Log the actionable reason the publisher is entering the dormant state. Called once on
/// the transition into dormancy, never per back-off re-check, so a long dormant stretch
/// does not spam the logs.
pub(crate) fn log_dormant_reason(outcome: &Result<Preflight, NymNodeError>) {
    match outcome {
        Ok(Preflight::Ready(_)) => {}
        Ok(Preflight::NotBonded) => error!(
            "directory publishing is idle: this node is not bonded (or is unbonding) in the mixnet contract - bond it to publish"
        ),
        Ok(Preflight::NotFundable) => error!(
            "directory publishing is idle: the node's chain account cannot fund writes - fund the account or set up a feegrant"
        ),
        Err(err) => {
            warn!("directory publishing preflight could not be completed: {err} - will retry")
        }
    }
}
