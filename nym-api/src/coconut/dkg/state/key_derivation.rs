// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::serde_helpers::recovered_keys;
use nym_coconut_dkg_common::types::DealingIndex;
use nym_dkg::{G2Projective, RecoveredVerificationKeys, Threshold};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

type ReceiverIndex = usize;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct KeyDerivationState {
    pub(crate) expected_threshold: Option<Threshold>,

    #[serde(with = "recovered_keys")]
    pub(crate) derived_partials: BTreeMap<DealingIndex, RecoveredVerificationKeys>,

    pub(crate) completed: bool,
    // because we couldn't decrypt shares or there were no shares, etc
    // failed:
}

impl KeyDerivationState {
    pub fn derived_partials_for(&self, receiver_index: ReceiverIndex) -> Option<Vec<G2Projective>> {
        let mut recovered = Vec::new();
        for keys in self.derived_partials.values() {
            // SAFETY:
            // make sure the receiver index of this receiver/dealer is within the size of the derived keys
            if keys.recovered_partials.len() <= receiver_index {
                return None;
            };
            recovered.push(keys.recovered_partials[receiver_index])
        }
        Some(recovered)
    }

    pub fn completed(&self) -> bool {
        self.completed
    }
}
