// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::state::DkgParticipant;
use coconut_dkg_common::types::{EpochId, Threshold};
use contracts_common::commitment::{Committable, DefaultHasher, Digest};
use dkg::NodeIndex;
use std::collections::BTreeMap;

type ReceiversDigest = Vec<u8>;

// not sure if this is the best place for it, but we can just move it later
// note that its an ephemeral type and thus the references in here rather than owned types
#[derive(Debug)]
pub(crate) struct CommittableEpochDealing<'a> {
    epoch_id: EpochId,
    system_threshold: Threshold,
    dealing_bytes: &'a [u8],
    // since all dealers are going to be using exactly the same set of receivers,
    // perform commitment on a hash of receivers so that we wouldn't need to recompute the bytes every time
    // we receive a dealing and want to verify the commitment
    receivers_digest: &'a ReceiversDigest,
}

impl<'a> CommittableEpochDealing<'a> {
    pub(crate) fn new(
        epoch_id: EpochId,
        system_threshold: Threshold,
        dealing_bytes: &'a [u8],
        receivers_digest: &'a ReceiversDigest,
    ) -> Self {
        CommittableEpochDealing {
            epoch_id,
            system_threshold,
            dealing_bytes,
            receivers_digest,
        }
    }
}

pub(crate) fn hash_receivers(receivers: &BTreeMap<NodeIndex, DkgParticipant>) -> ReceiversDigest {
    let mut bytes = Vec::new();
    // note: since it's a BTreeMap, we're guaranteed to always iterate in the same order over the values
    for receiver in receivers.values() {
        bytes.append(&mut receiver.to_bytes());
    }
    DefaultHasher::digest(bytes).to_vec()
}

impl<'a> Committable for CommittableEpochDealing<'a> {
    type DigestAlgorithm = DefaultHasher;

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.epoch_id.to_be_bytes());
        bytes.extend_from_slice(&self.system_threshold.to_be_bytes());
        bytes.extend_from_slice(self.dealing_bytes);
        bytes.extend_from_slice(self.receivers_digest);
        bytes
    }
}
