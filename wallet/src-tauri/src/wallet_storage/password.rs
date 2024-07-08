// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, Zeroizing};

// The `LoginId` is the top level id in the wallet file, and is not stored encrypted
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoginId(String);

impl LoginId {
    pub(crate) fn new(id: String) -> LoginId {
        LoginId(id)
    }
}

impl AsRef<str> for LoginId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<String> for LoginId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

impl From<&str> for LoginId {
    fn from(id: &str) -> Self {
        Self::new(id.to_string())
    }
}

impl fmt::Display for LoginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// For each encrypted login, we can have multiple encrypted accounts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub(crate) struct AccountId(String);

impl AccountId {
    pub(crate) fn new(id: String) -> AccountId {
        AccountId(id)
    }
}

impl AsRef<str> for AccountId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<String> for AccountId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

impl From<&str> for AccountId {
    fn from(id: &str) -> Self {
        Self::new(id.to_string())
    }
}

impl From<LoginId> for AccountId {
    fn from(login_id: LoginId) -> Self {
        Self::new(login_id.0)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// simple wrapper for String that will get zeroized on drop
pub type UserPassword = Zeroizing<String>;
