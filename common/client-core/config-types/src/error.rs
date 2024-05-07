// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
#[error("unable to upgrade config file from `{current_version}`")]
pub struct ConfigUpgradeFailure {
    pub current_version: String,
}

#[derive(Error, Debug)]
pub enum InvalidTrafficModeFailure {
    #[error("attempted to set medium toggle traffic mode with fast mode flag")]
    MediumToggleWithFastMode,

    #[error("attempted to set medium toggle traffic mode with no cover flag")]
    MediumToggleWithNoCover,
}
