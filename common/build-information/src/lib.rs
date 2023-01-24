// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: at a later date this crate should probably also expose `ContractBuildInformation`
// and be used by our smart contracts

#[derive(Debug)]
pub struct BinaryBuildInformation {
    // VERGEN_BUILD_TIMESTAMP
    /// Provides the build timestamp, for example `2021-02-23T20:14:46.558472672+00:00`.
    pub build_timestamp: &'static str,

    // VERGEN_BUILD_SEMVER
    /// Provides the build version, for example `0.1.0-9-g46f83e1`.
    pub build_version: &'static str,

    // VERGEN_GIT_SHA
    /// Provides the hash of the commit that was used for the build, for example `46f83e112520533338245862d366f6a02cef07d4`.
    pub commit_sha: &'static str,

    // VERGEN_GIT_COMMIT_TIMESTAMP
    /// Provides the timestamp of the commit that was used for the build, for example `2021-02-23T08:08:02-05:00`.
    pub commit_timestamp: &'static str,

    // VERGEN_GIT_BRANCH
    /// Provides the name of the git branch that was used for the build, for example `master`.
    pub commit_branch: &'static str,

    // VERGEN_RUSTC_SEMVER
    /// Provides the rustc version that was used for the build, for example `1.52.0-nightly`.
    pub rustc_version: &'static str,

    // VERGEN_RUSTC_CHANNEL
    /// Provides the rustc channel that was used for the build, for example `nightly`.
    pub rustc_channel: &'static str,

    // VERGEN_CARGO_PROFILE
    /// Provides the cargo profile that was used for the build, for example `debug`.
    pub cargo_profile: &'static str,
}

impl BinaryBuildInformation {
    // explicitly require the build_version to be passed as it's binary specific
    pub const fn new(build_version: &'static str) -> Self {
        BinaryBuildInformation {
            build_timestamp: env!("VERGEN_BUILD_TIMESTAMP"),
            build_version,
            commit_sha: env!("VERGEN_GIT_SHA"),
            commit_timestamp: env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
            commit_branch: env!("VERGEN_GIT_BRANCH"),
            rustc_version: env!("VERGEN_RUSTC_SEMVER"),
            rustc_channel: env!("VERGEN_RUSTC_CHANNEL"),
            cargo_profile: env!("VERGEN_CARGO_PROFILE"),
        }
    }

    pub fn to_owned(&self) -> BinaryBuildInformationOwned {
        BinaryBuildInformationOwned {
            build_timestamp: self.build_timestamp.to_owned(),
            build_version: self.build_version.to_owned(),
            commit_sha: self.commit_sha.to_owned(),
            commit_timestamp: self.commit_timestamp.to_owned(),
            commit_branch: self.commit_branch.to_owned(),
            rustc_version: self.rustc_version.to_owned(),
            rustc_channel: self.rustc_channel.to_owned(),
            cargo_profile: self.cargo_profile.to_owned(),
        }
    }

    pub fn pretty_print(&self) -> String {
        format!(
            r#"
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
"#,
            "Build Timestamp:",
            self.build_timestamp,
            "Build Version:",
            self.build_version,
            "Commit SHA:",
            self.commit_sha,
            "Commit Date:",
            self.commit_timestamp,
            "Commit Branch:",
            self.commit_branch,
            "rustc Version:",
            self.rustc_version,
            "rustc Channel:",
            self.rustc_channel,
            "cargo Profile:",
            self.cargo_profile,
        )
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BinaryBuildInformationOwned {
    // VERGEN_BUILD_TIMESTAMP
    /// Provides the build timestamp, for example `2021-02-23T20:14:46.558472672+00:00`.
    pub build_timestamp: String,

    // VERGEN_BUILD_SEMVER
    /// Provides the build version, for example `0.1.0-9-g46f83e1`.
    pub build_version: String,

    // VERGEN_GIT_SHA
    /// Provides the hash of the commit that was used for the build, for example `46f83e112520533338245862d366f6a02cef07d4`.
    pub commit_sha: String,

    // VERGEN_GIT_COMMIT_TIMESTAMP
    /// Provides the timestamp of the commit that was used for the build, for example `2021-02-23T08:08:02-05:00`.
    pub commit_timestamp: String,

    // VERGEN_GIT_BRANCH
    /// Provides the name of the git branch that was used for the build, for example `master`.
    pub commit_branch: String,

    // VERGEN_RUSTC_SEMVER
    /// Provides the rustc version that was used for the build, for example `1.52.0-nightly`.
    pub rustc_version: String,

    // VERGEN_RUSTC_CHANNEL
    /// Provides the rustc channel that was used for the build, for example `nightly`.
    pub rustc_channel: String,

    // VERGEN_CARGO_PROFILE
    /// Provides the cargo profile that was used for the build, for example `debug`.
    pub cargo_profile: String,
}
