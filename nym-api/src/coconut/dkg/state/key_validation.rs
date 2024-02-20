// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type ProposalId = u64;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct ValidationState {
    pub votes: HashMap<ProposalId, bool>,

    pub completed: bool,
}

impl ValidationState {
    /// Specifies whether this dealer has already registered in the particular DKG epoch
    pub fn completed(&self) -> bool {
        self.completed
    }
}
