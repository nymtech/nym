// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use http::HeaderValue;
use nym_bin_common::build_information::{BinaryBuildInformation, BinaryBuildInformationOwned};

#[derive(Clone, Debug)]
pub struct UserAgent {
    pub application: String,
    pub version: String,
    pub system: Option<String>,
    pub platform: String,
    pub git_commit: String,
}

impl fmt::Display for UserAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abbreviated_commit = self.git_commit.chars().take(7).collect::<String>();
        let system_information = self.system.clone().unwrap_or_else(|| "unknown".to_string());
        write!(
            f,
            "{}/{}/{}/{}/{}",
            self.application, self.version, system, self.platform, abbreviated_commit
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
            version: build_info.build_version.to_string(),
            system: None,
            platform: build_info.cargo_triple.to_string(),
            git_commit: build_info.commit_sha.to_string(),
        }
    }
}

impl From<BinaryBuildInformationOwned> for UserAgent {
    fn from(build_info: BinaryBuildInformationOwned) -> Self {
        UserAgent {
            application: build_info.binary_name,
            version: build_info.build_version,
            system: None,
            platform: build_info.cargo_triple,
            git_commit: build_info.commit_sha,
        }
    }
}
