// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::bip32::DerivationPath;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use zeroize::Zeroize;
use zeroize::Zeroizing;

use crate::error::BackendError;

use super::encryption::EncryptedData;
use super::password::WalletAccountId;
use super::UserPassword;

const CURRENT_WALLET_FILE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct StoredWallet {
  version: u32,
  accounts: Vec<EncryptedAccount>,
}

impl StoredWallet {
  pub fn version(&self) -> u32 {
    self.version
  }

  pub fn is_empty(&self) -> bool {
    self.accounts.is_empty()
  }

  pub fn len(&self) -> usize {
    self.accounts.len()
  }

  pub fn encrypted_account_by_index(&self, index: usize) -> Option<&EncryptedAccount> {
    self.accounts.get(index)
  }

  fn encrypted_account(
    &self,
    id: &WalletAccountId,
  ) -> Result<&EncryptedData<StoredAccount>, BackendError> {
    self
      .accounts
      .iter()
      .find(|account| &account.id == id)
      .map(|account| &account.account)
      .ok_or(BackendError::NoSuchIdInWallet)
  }

  pub fn add_encrypted_account(
    &mut self,
    new_account: EncryptedAccount,
  ) -> Result<(), BackendError> {
    if self.encrypted_account(&new_account.id).is_ok() {
      return Err(BackendError::IdAlreadyExistsInWallet);
    }
    self.accounts.push(new_account);
    Ok(())
  }

  pub fn decrypt_account(
    &self,
    id: &WalletAccountId,
    password: &UserPassword,
  ) -> Result<StoredAccount, BackendError> {
    self.encrypted_account(id)?.decrypt_struct(password)
  }

  pub fn decrypt_all(&self, password: &UserPassword) -> Result<Vec<StoredAccount>, BackendError> {
    self
      .accounts
      .iter()
      .map(|account| account.account.decrypt_struct(password))
      .collect::<Result<Vec<_>, _>>()
  }

  pub fn password_can_decrypt_all(&self, password: &UserPassword) -> bool {
    self.decrypt_all(password).is_ok()
  }
}

impl Default for StoredWallet {
  fn default() -> Self {
    StoredWallet {
      version: CURRENT_WALLET_FILE_VERSION,
      accounts: Vec::new(),
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EncryptedAccount {
  pub id: WalletAccountId,
  pub account: EncryptedData<StoredAccount>,
}

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

  // If we add accounts backed by something that is not a mnemonic, this should probably be changed
  // to return `Option<..>`.
  pub(crate) fn mnemonic(&self) -> &bip39::Mnemonic {
    match self {
      StoredAccount::Mnemonic(account) => account.mnemonic(),
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MnemonicAccount {
  mnemonic: bip39::Mnemonic,
  #[serde(with = "display_hd_path")]
  hd_path: DerivationPath,
}

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
