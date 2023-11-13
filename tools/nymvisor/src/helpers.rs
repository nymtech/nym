// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use std::fs;
use std::path::Path;
use tracing::trace;

pub(crate) fn init_path<P: AsRef<Path>>(path: P) -> Result<(), NymvisorError> {
    let path = path.as_ref();
    trace!("initialising {}", path.display());

    fs::create_dir_all(path).map_err(|source| NymvisorError::PathInitFailure {
        path: path.to_path_buf(),
        source,
    })
}
