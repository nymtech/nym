// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymvisorError;
use crate::error::NymvisorError::DaemonBuildInformationParseFailure;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::os::unix::prelude::ExitStatusExt;
use std::path::Path;

// each of our nym binaries (that are supported by `nymvisor`) expose `build-info` command
// that outputs the build information
pub(crate) fn get_daemon_build_information<P: AsRef<Path>>(
    executable_path: P,
) -> Result<BinaryBuildInformationOwned, NymvisorError> {
    let path = executable_path.as_ref();

    // TODO: do we need any timeouts here or could we just assume this is not going to take an eternity to execute?
    // I'm leaning towards the former
    let raw = std::process::Command::new(path)
        .args(["--no-banner", "build-info", "--output=json"])
        .output()
        .map_err(|source| NymvisorError::DaemonBuildInformationFailure {
            binary_path: path.to_path_buf(),
            source,
        })?;

    if !raw.status.success() {
        return Err(NymvisorError::DaemonExecutionFailure {
            exit_code: raw.status.code(),
            signal_code: raw.status.signal(),
            core_dumped: raw.status.core_dumped(),
        });
    }

    serde_json::from_slice(&raw.stdout)
        .map_err(|source| DaemonBuildInformationParseFailure { source })
}
