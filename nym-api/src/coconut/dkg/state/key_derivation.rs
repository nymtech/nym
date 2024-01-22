// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_dkg::Threshold;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct KeyDerivationState {
    pub(crate) expected_threshold: Option<Threshold>, // pub(crate) completed: bool,

    pub(crate) completed: bool,
    // because we couldn't decrypt shares or there were no shares, etc
    // failed:
}

impl KeyDerivationState {
    pub fn completed(&self) -> bool {
        self.completed
    }
}
