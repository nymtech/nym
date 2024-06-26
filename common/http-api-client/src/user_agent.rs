// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use http::HeaderValue;

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
        write!(
            f,
            "{}/{}/{}/{}",
            self.application, self.platform, self.version, self.git_commit
        )
    }
}

impl Into<HeaderValue> for UserAgent {
    fn into(self) -> HeaderValue {
        HeaderValue::from_str(&self.to_string()).unwrap()
    }
}
