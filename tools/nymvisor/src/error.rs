// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_file_watcher::NotifyError;
use nix::sys::signal::Signal;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::ffi::OsString;
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::process::ExitStatus;
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

    #[error("the value provided for environmental Url '{variable}': '{value}' is not a valid number: {source}")]
    MalformedUrlEnvVariable {
        variable: String,
        value: String,
        #[source]
        source: url::ParseError,
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

    #[error("the daemon execution has experienced an io failure: {source}")]
    DaemonIoFailure {
        #[source]
        source: io::Error,
    },

    #[error("there was already a genesis binary present for {daemon_name} which was different that the one provided.\nProvided:\n{provided_genesis:#?}\nExisting:\n{existing_info:#?}")]
    DuplicateDaemonGenesisBinary {
        daemon_name: String,
        existing_info: Box<BinaryBuildInformationOwned>,
        provided_genesis: Box<BinaryBuildInformationOwned>,
    },

    #[error("there was already a symlink for the 'current' binary of {daemon_name}. it's pointing to {} while we needed to create one to {}", link.display(), expected_link.display())]
    ExistingCurrentSymlink {
        daemon_name: String,
        link: PathBuf,
        expected_link: PathBuf,
    },

    #[error("failed to send to send {signal} to the daemon process: {source}")]
    DaemonSignalFailure {
        signal: Signal,
        #[source]
        source: nix::Error,
    },

    #[error("failed to watch for changes in the upgrade-plan.json: {source}")]
    UpgradePlanFileWatchFailure {
        #[from]
        source: NotifyError,
    },
}

impl From<ExitStatus> for NymvisorError {
    fn from(value: ExitStatus) -> Self {
        use std::os::unix::prelude::ExitStatusExt;

        assert!(!value.success());
        NymvisorError::DaemonExecutionFailure {
            exit_code: value.code(),
            signal_code: value.signal(),
            core_dumped: value.core_dumped(),
        }
    }
}
