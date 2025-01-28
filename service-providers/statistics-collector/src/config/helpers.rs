// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use log::trace;
use std::path::Path;

use crate::error::StatsCollectorError;

pub async fn try_upgrade_config<P: AsRef<Path>>(
    _config_path: P,
) -> Result<(), StatsCollectorError> {
    trace!("Attempting to upgrade config");

    Ok(())
}
