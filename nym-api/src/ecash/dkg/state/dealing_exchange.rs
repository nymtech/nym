// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::serde_helpers::generated_dealings;
use crate::ecash::dkg::state::DkgParticipant;
use nym_coconut_dkg_common::types::DealingIndex;
use nym_dkg::{Dealing, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct DealingExchangeState {
    pub(crate) dealers: BTreeMap<NodeIndex, DkgParticipant>,

    #[serde(with = "generated_dealings")]
    pub(crate) generated_dealings: HashMap<DealingIndex, Dealing>,

    pub(crate) receiver_index: Option<usize>,

    pub(crate) completed: bool,
}

impl DealingExchangeState {
    /// Specifies whether this dealer has already shared dealings in this DKG epoch
    pub fn completed(&self) -> bool {
        self.completed
    }
}
