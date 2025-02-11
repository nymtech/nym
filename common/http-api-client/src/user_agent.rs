// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{fmt, str::FromStr};

use http::HeaderValue;
use nym_bin_common::build_information::{BinaryBuildInformation, BinaryBuildInformationOwned};
use serde::{Deserialize, Serialize};

/// Strut containing characteristic elements sent to the API providing basic context information of the requesting client.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserAgent {
    /// The internal crate / application / subsystem making use of API client
    pub application: String,
    /// version of the calling crate / application / subsystem
    pub version: String,
    /// client platform
    pub platform: String,
    /// source commit version for the calling calling crate / subsystem
    pub git_commit: String,
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("invalid user agent string: {0}")]
pub struct UserAgentError(String);

impl FromStr for UserAgent {
    type Err = UserAgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 4 {
            return Err(UserAgentError(s.to_string()));
        }

        Ok(UserAgent {
            application: parts[0].to_string(),
            version: parts[1].to_string(),
            platform: parts[2].to_string(),
            git_commit: parts[3].to_string(),
        })
    }
}

impl TryFrom<&str> for UserAgent {
    type Error = UserAgentError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        UserAgent::from_str(s)
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
            version: build_info.build_version.to_string(),
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
            platform: build_info.cargo_triple,
            git_commit: build_info.commit_sha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_valid_user_agent() {
        let user_agent = "nym-mixnode/0.11.0/x86_64-unknown-linux-gnu/abcdefg";
        let parsed = UserAgent::from_str(user_agent).unwrap();
        assert_eq!(
            parsed,
            UserAgent {
                application: "nym-mixnode".to_string(),
                version: "0.11.0".to_string(),
                platform: "x86_64-unknown-linux-gnu".to_string(),
                git_commit: "abcdefg".to_string()
            }
        );
    }

    #[test]
    fn parsing_invalid_user_agent() {
        let user_agent = "nym-mixnode/0.11.0/x86_64-unknown-linux-gnu";
        assert!(UserAgent::from_str(user_agent).is_err());
    }

    #[test]
    fn converting_user_agent_to_string() {
        let user_agent = UserAgent {
            application: "nym-mixnode".to_string(),
            version: "0.11.0".to_string(),
            platform: "x86_64-unknown-linux-gnu".to_string(),
            git_commit: "abcdefg".to_string(),
        };

        assert_eq!(
            user_agent.to_string(),
            "nym-mixnode/0.11.0/x86_64-unknown-linux-gnu/abcdefg"
        );
    }

    #[test]
    fn converting_user_agent_to_display() {
        let user_agent = UserAgent {
            application: "nym-mixnode".to_string(),
            version: "0.11.0".to_string(),
            platform: "x86_64-unknown-linux-gnu".to_string(),
            git_commit: "abcdefg".to_string(),
        };

        assert_eq!(
            format!("{}", user_agent),
            "nym-mixnode/0.11.0/x86_64-unknown-linux-gnu/abcdefg"
        );
    }

    #[test]
    fn converting_user_agent_to_header_value_fails() {
        let user_agent = UserAgent {
            application: "nym-mixnode".to_string(),
            version: "0.11.0".to_string(),
            platform: "x86_64-unknown-linux-gnu".to_string(),
            git_commit: "abcdefg".to_string(),
        };

        let header_value: Result<HeaderValue, _> = user_agent.clone().try_into();
        assert!(header_value.is_ok());
    }

    #[test]
    fn converting_user_agent_to_header_value_has_same_string_representation() {
        let user_agent = UserAgent {
            application: "nym-mixnode".to_string(),
            version: "0.11.0".to_string(),
            platform: "x86_64-unknown-linux-gnu".to_string(),
            git_commit: "abcdefg".to_string(),
        };

        let header_value: HeaderValue = user_agent.clone().try_into().unwrap();
        assert_eq!(header_value.to_str().unwrap(), user_agent.to_string());
    }
}
