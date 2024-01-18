// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::serde_helpers::generated_dealings;
use nym_coconut_dkg_common::types::DealingIndex;
use nym_dkg::{Dealing, NodeIndex};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DealingExchangeState {
    // pub(crate) assigned_index: Option<NodeIndex>,
    #[serde(with = "generated_dealings")]
    pub(crate) generated_dealings: HashMap<DealingIndex, Dealing>,

    pub(crate) completed: bool,
}

impl DealingExchangeState {
    /// Specifies whether this dealer has already shared dealings in this DKG epoch
    pub fn completed(&self) -> bool {
        self.completed
    }
}
