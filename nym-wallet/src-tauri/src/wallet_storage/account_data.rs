// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// The wallet storage is a single json file, containing multiple entries. These are referred to as
// Logins, and has a plaintext id tag attached.
//
// Each encrypted login contains either a single account, or a list of multiple accounts.
//
// NOTE: A not insignificant amount of complexity comes from being able to handle both these cases,
// instead of, for example, converting a single account to a list of multiple accounts with a single
// entry. This also avoids resaving the wallet file when opening a file created with an earlier
// version of the wallet.
//
// In the future we might want to simplify by dropping the support for a single account entry,
// instead treating as muliple accounts with one entry.

use cosmrs::bip32::DerivationPath;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::BackendError;

use super::encryption::EncryptedData;
use super::password::AccountId;
use super::UserPassword;

const CURRENT_WALLET_FILE_VERSION: u32 = 1;

/// The wallet, stored as a serialized json file.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct StoredWallet {
  version: u32,
  accounts: Vec<EncryptedLogin>,
}

impl StoredWallet {
  #[allow(unused)]
  pub fn version(&self) -> u32 {
    self.version
  }

  #[allow(unused)]
  pub fn len(&self) -> usize {
    self.accounts.len()
  }

  pub fn is_empty(&self) -> bool {
    self.accounts.is_empty()
  }

  pub fn add_encrypted_login(&mut self, new_login: EncryptedLogin) -> Result<(), BackendError> {
    if self.get_encrypted_login(&new_login.id).is_ok() {
      return Err(BackendError::IdAlreadyExistsInWallet);
    }
    self.accounts.push(new_login);
    Ok(())
  }

  fn get_encrypted_login(
    &self,
    id: &AccountId,
  ) -> Result<&EncryptedData<StoredLogin>, BackendError> {
    self
      .accounts
      .iter()
      .find(|account| &account.id == id)
      .map(|account| &account.account)
      .ok_or(BackendError::NoSuchIdInWallet)
  }

  fn get_encrypted_login_mut(
    &mut self,
    id: &AccountId,
  ) -> Result<&mut EncryptedLogin, BackendError> {
    self
      .accounts
      .iter_mut()
      .find(|account| &account.id == id)
      //.map(|account| &mut account.account)
      .ok_or(BackendError::NoSuchIdInWallet)
  }

  #[cfg(test)]
  pub fn get_encrypted_login_by_index(&self, index: usize) -> Option<&EncryptedLogin> {
    self.accounts.get(index)
  }

  pub fn replace_encrypted_login(&mut self, new_login: EncryptedLogin) -> Result<(), BackendError> {
    let login = self.get_encrypted_login_mut(&new_login.id)?;
    *login = new_login;
    Ok(())
  }

  pub fn remove_encrypted_login(&mut self, id: &AccountId) -> Option<EncryptedLogin> {
    if let Some(index) = self.accounts.iter().position(|account| &account.id == id) {
      log::info!("Removing from wallet file: {id}");
      Some(self.accounts.remove(index))
    } else {
      log::debug!("Tried to remove non-existent id from wallet: {id}");
      None
    }
  }

  pub fn decrypt_login(
    &self,
    id: &AccountId,
    password: &UserPassword,
  ) -> Result<StoredLogin, BackendError> {
    self.get_encrypted_login(id)?.decrypt_struct(password)
  }

  pub fn decrypt_all(&self, password: &UserPassword) -> Result<Vec<StoredLogin>, BackendError> {
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

/// Each entry in the stored wallet file. An id field in plaintext and an encrypted stored login.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EncryptedLogin {
  pub id: AccountId,
  pub account: EncryptedData<StoredLogin>,
}

/// A stored login is either a account, such as a mnemonic, or a list of multiple accounts where
/// each has an inner id. Future proofed for having private key backed accounts.
#[derive(Serialize, Deserialize, Debug, Zeroize)]
#[serde(untagged)]
#[zeroize(drop)]
pub(crate) enum StoredLogin {
  Mnemonic(MnemonicAccount),
  // PrivateKey(PrivateKeyAccount)
  Multiple(MultipleAccounts),
}

impl StoredLogin {
  pub(crate) fn new_mnemonic_backed_account(
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
  ) -> Self {
    Self::Mnemonic(MnemonicAccount { mnemonic, hd_path })
  }

  pub(crate) fn new_multiple_login() -> Self {
    Self::Multiple(MultipleAccounts::empty())
  }

  #[cfg(test)]
  pub(crate) fn as_mnemonic_account(&self) -> Option<&MnemonicAccount> {
    match self {
      StoredLogin::Mnemonic(mn) => Some(mn),
      StoredLogin::Multiple(_) => None,
    }
  }

  #[cfg(test)]
  pub(crate) fn as_multiple_accounts(&self) -> Option<&MultipleAccounts> {
    match self {
      StoredLogin::Mnemonic(_) => None,
      StoredLogin::Multiple(accounts) => Some(accounts),
    }
  }

  pub(crate) fn unwrap_into_multiple_accounts(self, id: AccountId) -> MultipleAccounts {
    match self {
      StoredLogin::Mnemonic(ref account) => account.clone().into_multiple(id),
      StoredLogin::Multiple(ref accounts) => accounts.clone(),
    }
  }
}

/// An account backed by a unique mnemonic.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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

  pub(crate) fn into_wallet_account(self, id: AccountId) -> WalletAccount {
    WalletAccount {
      id,
      account: self.into(),
    }
  }

  pub(crate) fn into_multiple(self, id: AccountId) -> MultipleAccounts {
    MultipleAccounts::new(self.into_wallet_account(id))
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

/// Multiple stored accounts, each entry having an id and a data field.
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, PartialEq, Eq)]
pub(crate) struct MultipleAccounts {
  accounts: Vec<WalletAccount>,
}

impl MultipleAccounts {
  pub(crate) fn empty() -> Self {
    MultipleAccounts {
      accounts: Vec::new(),
    }
  }

  pub(crate) fn new(account: WalletAccount) -> Self {
    MultipleAccounts {
      accounts: vec![account],
    }
  }

  pub(crate) fn get_accounts(&self) -> impl Iterator<Item = &WalletAccount> {
    self.accounts.iter()
  }

  pub(crate) fn get_account(&self, id: &AccountId) -> Option<&WalletAccount> {
    self.accounts.iter().find(|account| &account.id == id)
  }

  pub(crate) fn into_accounts(self) -> impl Iterator<Item = WalletAccount> {
    self.accounts.into_iter()
  }

  #[allow(unused)]
  pub(crate) fn len(&self) -> usize {
    self.accounts.len()
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.accounts.is_empty()
  }

  pub(crate) fn add(
    &mut self,
    id: AccountId,
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
  ) -> Result<(), BackendError> {
    if self.get_account(&id).is_some() {
      Err(BackendError::IdAlreadyExistsInStoredWalletLogin)
    } else {
      self
        .accounts
        .push(WalletAccount::new_mnemonic_backed_account(
          id, mnemonic, hd_path,
        ));
      Ok(())
    }
  }

  pub(crate) fn remove(&mut self, id: &AccountId) -> Result<(), BackendError> {
    if self.get_account(id).is_none() {
      return Err(BackendError::NoSuchIdInWalletLoginEntry);
    }
    self.accounts.retain(|accounts| &accounts.id != id);
    Ok(())
  }
}

impl From<Vec<WalletAccount>> for MultipleAccounts {
  fn from(accounts: Vec<WalletAccount>) -> MultipleAccounts {
    Self { accounts }
  }
}

/// An entry in the list of stored accounts
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, PartialEq, Eq)]
pub(crate) struct WalletAccount {
  pub id: AccountId,
  pub account: AccountData,
}

impl WalletAccount {
  pub(crate) fn new_mnemonic_backed_account(
    id: AccountId,
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
  ) -> Self {
    Self {
      id,
      account: AccountData::new_mnemonic_backed_account(mnemonic, hd_path),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, PartialEq, Eq)]
#[serde(untagged)]
#[zeroize(drop)]
pub(crate) enum AccountData {
  Mnemonic(MnemonicAccount),
  // PrivateKey(PrivateKeyAccount)
}

impl AccountData {
  pub(crate) fn new_mnemonic_backed_account(
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
  ) -> AccountData {
    AccountData::Mnemonic(MnemonicAccount { mnemonic, hd_path })
  }

  pub(crate) fn mnemonic(&self) -> &bip39::Mnemonic {
    match self {
      AccountData::Mnemonic(account) => account.mnemonic(),
    }
  }

  #[cfg(test)]
  pub(crate) fn hd_path(&self) -> &DerivationPath {
    match self {
      AccountData::Mnemonic(account) => account.hd_path(),
    }
  }
}

impl From<MnemonicAccount> for AccountData {
  fn from(mnemonic_account: MnemonicAccount) -> Self {
    AccountData::Mnemonic(mnemonic_account)
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
