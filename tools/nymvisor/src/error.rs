// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ffi::OsString;
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum NymvisorError {
    #[error(
    "failed to load config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to initialise the path '{}': {source}", path.display())]
    PathInitFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the provided env file was malformed: {source}")]
    MalformedEnvFile {
        #[from]
        source: dotenvy::Error,
    },

    #[error("the value provided for environmental variable '{variable}' was not valid unicode: {value:?}")]
    MalformedEnvVariable { variable: String, value: OsString },

    #[error("the value provided for environmental boolean variable '{variable}': '{value}' is not a valid boolean")]
    MalformedBoolEnvVariable { variable: String, value: String },

    #[error("the value provided for environmental duration variable '{variable}': '{value}' is not a valid duration: {source}")]
    MalformedDurationEnvVariable {
        variable: String,
        value: String,
        #[source]
        source: humantime::DurationError,
    },

    #[error("the value provided for environmental numerical variable '{variable}': '{value}' is not a valid number: {source}")]
    MalformedNumberEnvVariable {
        variable: String,
        value: String,
        #[source]
        source: ParseIntError,
    },

    #[error("failed to copy daemon binary from '{}' to '{}': {source}", source_path.display(), target_path.display())]
    DaemonBinaryCopyFailure {
        source_path: PathBuf,
        target_path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to create symlink from '{}' to '{}': {source}", source_path.display(), target_path.display())]
    SymlinkCreationFailure {
        source_path: PathBuf,
        target_path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the value of daemon home has to be provided by either `--daemon-home` flag or `$DAEMON_HOME` environmental variable")]
    DaemonHomeUnavailable,

    #[error("could not identify nymvisor instance. please specify either $NYMVISOR_CONFIG_PATH, $NYMVISOR_ID or $DAEMON_NAME")]
    UnknownNymvisorInstance,

    #[error("failed to obtain build information from the daemon executable ('{}'): {source}", binary_path.display())]
    DaemonBuildInformationFailure {
        binary_path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse build information from the daemon executable: {source}")]
    DaemonBuildInformationParseFailure {
        #[source]
        source: serde_json::Error,
    },

    #[error("the daemon execution has failed with the following exit code: {exit_code:?}. the associated signal code: {signal_code:?}. the core was dumped: {core_dumped}")]
    DaemonExecutionFailure {
        // exit code of the process, if any
        exit_code: Option<i32>,

        // if the process was WIFSIGNALED, this returns WTERMSIG.
        signal_code: Option<i32>,
        core_dumped: bool,
    },
}
