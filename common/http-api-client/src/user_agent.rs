// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use http::HeaderValue;
use nym_bin_common::build_information::{BinaryBuildInformation, BinaryBuildInformationOwned};

#[derive(Clone, Debug)]
pub struct UserAgent {
    application: String,
    platform: String,
    version: String,
    git_commit: String,
}

impl UserAgent {
    pub fn new(application: String, platform: String, version: String, git_commit: String) -> Self {
        UserAgent {
            application,
            platform,
            version,
            git_commit,
        }
    }
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abbreviated_commit = self.git_commit.chars().take(7).collect::<String>();
        write!(
            f,
            "{}/{}/{}/{}",
            self.application, self.version, self.platform, abbreviated_commit
        )
    }
}

impl TryFrom<UserAgent> for HeaderValue {
    type Error = http::header::InvalidHeaderValue;

    fn try_from(user_agent: UserAgent) -> Result<Self, Self::Error> {
        HeaderValue::from_str(&user_agent.to_string())
    }
}

impl From<BinaryBuildInformation> for UserAgent {
    fn from(build_info: BinaryBuildInformation) -> Self {
        UserAgent {
            application: build_info.binary_name.to_string(),
            platform: build_info.cargo_triple.to_string(),
            version: build_info.build_version.to_string(),
            git_commit: build_info.commit_sha.to_string(),
        }
    }
}

impl From<BinaryBuildInformationOwned> for UserAgent {
    fn from(build_info: BinaryBuildInformationOwned) -> Self {
        UserAgent {
            application: build_info.binary_name,
            platform: build_info.cargo_triple,
            version: build_info.build_version,
            git_commit: build_info.commit_sha,
        }
    }
}
