// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::bip32::DerivationPath;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use zeroize::Zeroize;
use zeroize::Zeroizing;

// future-proofing
#[derive(Serialize, Deserialize, Debug, Zeroize)]
#[serde(untagged)]
#[zeroize(drop)]
pub(crate) enum StoredAccount {
  Mnemonic(MnemonicAccount),
  // PrivateKey(PrivateKeyAccount)
}

impl StoredAccount {
  pub(crate) fn new_mnemonic_backed_account(
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
  ) -> StoredAccount {
    StoredAccount::Mnemonic(MnemonicAccount { mnemonic, hd_path })
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MnemonicAccount {
  mnemonic: bip39::Mnemonic,
  #[serde(with = "display_hd_path")]
  hd_path: DerivationPath,
}

// we only ever want to expose those getters in the test code
#[cfg(test)]
impl MnemonicAccount {
  pub(crate) fn mnemonic(&self) -> &bip39::Mnemonic {
    &self.mnemonic
  }

  pub(crate) fn hd_path(&self) -> &DerivationPath {
    &self.hd_path
  }
}

impl Zeroize for MnemonicAccount {
  fn zeroize(&mut self) {
    // in ideal world, Mnemonic would have had zeroize defined on it (there's an almost year old PR that introduces it)
    // and the memory would have been filled with zeroes.
    //
    // we really don't want to keep our real mnemonic in memory, so let's do the semi-nasty thing
    // of overwriting it with a fresh mnemonic that was never used before
    //
    // note: this function can only fail on an invalid word count, which clearly is not the case here
    self.mnemonic = bip39::Mnemonic::generate(self.mnemonic.word_count()).unwrap();

    // further note: we don't really care about the hd_path, there's nothing secret about it.
  }
}

impl Drop for MnemonicAccount {
  fn drop(&mut self) {
    self.zeroize()
  }
}

mod display_hd_path {
  use cosmrs::bip32::DerivationPath;
  use serde::{Deserialize, Deserializer, Serializer};

  pub fn serialize<S: Serializer>(
    hd_path: &DerivationPath,
    serializer: S,
  ) -> Result<S::Ok, S::Error> {
    serializer.collect_str(hd_path)
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
  ) -> Result<DerivationPath, D::Error> {
    let s = <&str>::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
  }
}
