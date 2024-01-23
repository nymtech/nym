// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FinalizationState {}

impl FinalizationState {
    /// Specifies whether this dealer has already registered in the particular DKG epoch
    pub fn completed(&self) -> bool {
        todo!()
    }
}
