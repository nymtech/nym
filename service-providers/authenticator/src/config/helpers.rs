// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::trace;
use std::path::Path;

use crate::error::AuthenticatorError;

pub async fn try_upgrade_config<P: AsRef<Path>>(_config_path: P) -> Result<(), AuthenticatorError> {
    trace!("Attempting to upgrade config");

    Ok(())
}
