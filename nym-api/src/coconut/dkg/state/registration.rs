// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_dkg::NodeIndex;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RegistrationState {
    pub(crate) assigned_index: Option<NodeIndex>,
}

impl RegistrationState {
    /// Specifies whether this dealer has already registered in the particular DKG epoch
    pub fn completed(&self) -> bool {
        self.assigned_index.is_some()
    }
}
