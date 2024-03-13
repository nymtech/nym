// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymNodeError;
use std::path::Path;

pub(crate) async fn try_load_current_config<P: AsRef<Path>>(
    path: P,
) -> Result<Config, NymNodeError> {
    todo!()
}
