// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::state::DkgParticipant;
use coconut_dkg_common::types::EpochId;
use contracts_common::commitment::{Committable, DefaultHasher, Digest};
use dkg::{Dealing, NodeIndex};
use std::collections::BTreeMap;

type ReceiversDigest = Vec<u8>;

// not sure if this is the best place for it, but we can just move it later
// note that its an ephemeral type and thus the references in here rather than owned types
pub(crate) struct CommittableEpochDealing<'a> {
    epoch_id: EpochId,
    dealing: &'a Dealing,
    // since all dealers are going to be using exactly the same set of receivers,
    // perform commitment on a hash of receivers so that you wouldn't need to recompute the bytes every time
    // you receive a dealing and verify the commitment
    receivers: &'a ReceiversDigest,
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
        todo!()
    }
}
