// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FinalizationState {
    pub(crate) completed: bool,
}

impl FinalizationState {
    /// Specifies whether this (or another) dealer has already executed its verification proposal
    pub fn completed(&self) -> bool {
        self.completed
    }
}
