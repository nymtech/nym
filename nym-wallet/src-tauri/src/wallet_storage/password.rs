// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

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

impl fmt::Display for AccountId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

// simple wrapper for String that will get zeroized on drop
#[derive(Zeroize)]
#[zeroize(drop)]
pub(crate) struct UserPassword(String);

impl UserPassword {
  pub(crate) fn new(pass: String) -> UserPassword {
    UserPassword(pass)
  }

  pub(crate) fn as_bytes(&self) -> &[u8] {
    self.0.as_bytes()
  }
}

impl AsRef<str> for UserPassword {
  fn as_ref(&self) -> &str {
    self.0.as_ref()
  }
}
