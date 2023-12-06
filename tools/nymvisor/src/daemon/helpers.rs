// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use crate::error::NymvisorError::DaemonBuildInformationParseFailure;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::fmt::Debug;
use std::os::unix::prelude::ExitStatusExt;
use std::path::Path;

// each of our nym binaries (that are supported by `nymvisor`) expose `build-info` command
// that outputs the build information
#[instrument]
pub(crate) fn get_daemon_build_information<P: AsRef<Path> + Debug>(
    executable_path: P,
) -> Result<BinaryBuildInformationOwned, NymvisorError> {
}
