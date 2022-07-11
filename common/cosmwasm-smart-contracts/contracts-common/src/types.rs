// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

// TODO: there's no reason this couldn't be used for proper binaries, but in that case
// perhaps the struct should get renamed and moved to a "more" common crate
#[derive(Debug, Serialize, Deserialize)]
pub struct ContractBuildInformation {
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
}
