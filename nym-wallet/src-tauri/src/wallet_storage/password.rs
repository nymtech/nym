// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use zeroize::Zeroize;

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
