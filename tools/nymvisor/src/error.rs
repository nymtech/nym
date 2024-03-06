// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::upgrades::types::{CurrentVersionInfo, DigestAlgorithm, UpgradeInfo};
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nym_async_file_watcher::NotifyError;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::ffi::OsString;
use std::io;
use std::num::ParseIntError;
use std::path::PathBuf;
use std::process::ExitStatus;
use thiserror::Error;
use url::Url;

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

    #[error(
    "failed to load upgrade info using path '{}'. detailed message: {source}", path.display()
    )]
    UpgradeInfoLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save upgrade info for upgrade '{name}' using path '{}'. detailed message: {source}", path.display()
    )]
    UpgradeInfoSaveFailure {
        name: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("there seem to be an upgrade-info.json file present without the associated binary for upgrade '{name}' at path {}", path.display())]
    UpgradeInfoWithNoBinary { name: String, path: PathBuf },

    #[error("there seem to already exist the upgrade-plan.json at {}, but it has mismatched current information. The expected current is {current_name} but the saved one specifies {existing_name}", path.display())]
    PreexistingUpgradePlan {
        path: PathBuf,
        current_name: String,
        existing_name: String,
    },

    #[error(
    "failed to load upgrade plan using path '{}'. detailed message: {source}", path.display()
    )]
    UpgradePlanLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save upgrade plan using path '{}'. detailed message: {source}", path.display()
    )]
    UpgradePlanSaveFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to load upgrade history using path '{}'. detailed message: {source}", path.display()
    )]
    UpgradeHistoryLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save upgrade history using path '{}'. detailed message: {source}", path.display()
    )]
    UpgradeHistorySaveFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to load current version information using path '{}'. detailed message: {source}", path.display()
    )]
    CurrentVersionInfoLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save current version information using path '{}'. detailed message: {source}", path.display()
    )]
    CurrentVersionInfoSaveFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "the current version information does not match the expected `current/upgrade-info.json`.\n\
The daemon version is:\n{current_version_info:#?}\n\
While the stored info point to:\n{current_info:#?}"
    )]
    UnexpectedCurrentVersionInfo {
        current_info: Box<UpgradeInfo>,
        current_version_info: Box<CurrentVersionInfo>,
    },

    #[error(
        "the current daemon build information does not match the expected stored value.\n\
The daemon build is:\n{daemon_info:#?}\n\
While the stored info point to:\n{stored_info:#?}"
    )]
    UnexpectedDaemonBuild {
        daemon_info: Box<BinaryBuildInformationOwned>,
        stored_info: Box<BinaryBuildInformationOwned>,
    },

    #[error("the daemon for upgrade '{upgrade_name}' has version {daemon_version} while {expected} was expected instead")]
    UnexpectedUpgradeDaemonVersion {
        upgrade_name: String,
        daemon_version: String,
        expected: String,
    },

    #[error("the provided daemon at {} is not a file", path.display())]
    DaemonNotAFile { path: PathBuf },

    #[error("could not read daemon's metadata at {}: {source}", path.display())]
    MetadataReadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not adjust permission of the daemon at: {}: {source}", path.display())]
    DaemonPermissionFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not acquire the lock at {} to perform binary upgrade with error code {libc_code}. It is either held by another process or this nymvisor has experienced a critical failure during previous upgrade attempt", lock_path.display())]
    UnableToAcquireUpgradePlanLock {
        lock_path: PathBuf,
        libc_code: Errno,
    },

    #[error("could not create the lock file at {} to perform binary upgrade: {source}", path.display())]
    LockFileCreationFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not remove the lock file at {} after performing binary upgrade: {source}", path.display())]
    LockFileRemovalFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not create the backup file at {} before performing binary upgrade: {source}", path.display())]
    BackupFileCreationFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not tar backup directory {} to {}: {source}", data_source.display(), path.display())]
    BackupTarDirFailure {
        path: PathBuf,
        data_source: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not tar backup file {} to {}: {source}", data_source.display(), path.display())]
    BackupTarFileFailure {
        path: PathBuf,
        data_source: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not finalize the tar backup file at {}: {source}", path.display())]
    BackupTarFinalizationFailure {
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

    #[error("failed to remove symlink at '{}': {source}", path.display())]
    SymlinkRemovalFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the value of daemon home has to be provided by either `--daemon-home` flag or `$DAEMON_HOME` environmental variable")]
    DaemonHomeUnavailable,

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

    #[error("there already exist upgrade binary for '{name}' at: {}. if you want to overwrite its content, use --force flag", path.display())]
    ExistingUpgrade { name: String, path: PathBuf },

    #[error("there already exist upgrade information for '{name}' at: {}. if you want to overwrite its content, use --force flag", path.display())]
    ExistingUpgradeInfo { name: String, path: PathBuf },

    #[error("the current upgrade-plan.json has planned upgrade for '{name}', but no corresponding upgrade-info.json file could be found")]
    UpgradePlanWithNoInfo { name: String },

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

    #[error("failed to query the upstream url ('{url}'): {source}")]
    UpstreamQueryFailure {
        url: Url,
        #[source]
        source: reqwest::Error,
    },

    #[error(
        "attempted to perform binary upgrade with no upgrades queued up in the upgrade plan file"
    )]
    NoQueuedUpgrades,

    #[error("could not find the upgrade binary at {} while the binary download is disabled", path.display())]
    NoUpgradeBinaryWithDisabledDownload { path: PathBuf },

    #[error("upgrade '{upgrade_name}' does not have any valid download URLs for the current arch '{arch}'. The available arches are: {available:?}")]
    NoDownloadUrls {
        upgrade_name: String,
        arch: String,
        available: Vec<String>,
    },

    #[error("failed to download the upgrade binary from '{url}': {source}")]
    UpgradeDownloadFailure {
        url: Url,
        #[source]
        source: reqwest::Error,
    },

    #[error("failed to verify checksum for upgrade '{upgrade_name}' using {algorithm}. Got '{encoded_checksum}' while expected '{expected_checksum}'")]
    DownloadChecksumFailure {
        upgrade_name: String,
        encoded_checksum: String,
        expected_checksum: String,
        algorithm: DigestAlgorithm,
    },

    #[error("could not calculate checksum for downloaded file at '{}': {source}", path.display())]
    ChecksumCalculationFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("download information for upgrade '{upgrade_name}' is missing checksum")]
    MissingDownloadChecksum { upgrade_name: String },

    #[error("failed to create daemon binary at {}: {source}", path.display())]
    DaemonBinaryCreationFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the daemon has reached the maximum number of startup failures ({failures})")]
    DaemonMaximumStartupFailures { failures: usize },

    #[error("the daemon restart on failure is disabled")]
    DisabledRestartOnFailure,

    #[error("failed to read directory content of nymvisor instances at {}: {source}", path.display())]
    InstancesReadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not load the default config file as there isn't a single nymvisor instance initiated (there are {instances}). please specify either $NYMVISOR_CONFIG_PATH or $NYMVISOR_ID")]
    NotSingleton { instances: usize },

    #[error("failed to crate tokio's runtime: {source}")]
    RuntimeCreationFailure { source: io::Error },
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
