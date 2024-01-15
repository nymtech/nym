// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use std::path::Path;

pub(crate) fn try_upgrade_config<P: AsRef<Path>>(config_path: P) -> Result<(), NymRewarderError> {
    let _ = config_path;
    Ok(())
}
