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

use super::encryption::EncryptedData;
use super::password::{AccountId, LoginId};
use super::UserPassword;
use crate::error::BackendError;
use bip39::Mnemonic;
use nym_validator_client::nyxd::bip32::DerivationPath;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

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
            return Err(BackendError::WalletLoginIdAlreadyExists);
        }
        self.accounts.push(new_login);
        Ok(())
    }

    fn get_encrypted_login(
        &self,
        id: &LoginId,
    ) -> Result<&EncryptedData<StoredLogin>, BackendError> {
        self.accounts
            .iter()
            .find(|account| &account.id == id)
            .map(|account| &account.account)
            .ok_or(BackendError::WalletNoSuchLoginId)
    }

    fn get_encrypted_login_mut(
        &mut self,
        id: &LoginId,
    ) -> Result<&mut EncryptedLogin, BackendError> {
        self.accounts
            .iter_mut()
            .find(|account| &account.id == id)
            .ok_or(BackendError::WalletNoSuchLoginId)
    }

    #[cfg(test)]
    pub fn get_encrypted_login_by_index(&self, index: usize) -> Option<&EncryptedLogin> {
        self.accounts.get(index)
    }

    pub fn replace_encrypted_login(
        &mut self,
        new_login: EncryptedLogin,
    ) -> Result<(), BackendError> {
        let login = self.get_encrypted_login_mut(&new_login.id)?;
        *login = new_login;
        Ok(())
    }

    pub fn remove_encrypted_login(&mut self, id: &LoginId) -> Option<EncryptedLogin> {
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
        id: &LoginId,
        password: &UserPassword,
    ) -> Result<StoredLogin, BackendError> {
        self.get_encrypted_login(id)?.decrypt_struct(password)
    }

    pub fn reencrypt_all(
        &mut self,
        current_password: &UserPassword,
        new_password: &UserPassword,
    ) -> Result<(), BackendError> {
        if current_password == new_password {
            return Ok(());
        }
        for encrypted_login in &mut self.accounts {
            let login = encrypted_login.account.decrypt_struct(current_password)?;
            *encrypted_login =
                EncryptedLogin::encrypt(encrypted_login.id.clone(), &login, new_password)?;
        }
        Ok(())
    }

    pub fn decrypt_all(&self, password: &UserPassword) -> Result<Vec<StoredLogin>, BackendError> {
        self.accounts
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
    pub id: LoginId,
    pub account: EncryptedData<StoredLogin>,
}

impl EncryptedLogin {
    pub(crate) fn encrypt(
        id: LoginId,
        login: &StoredLogin,
        password: &UserPassword,
    ) -> Result<Self, BackendError> {
        Ok(EncryptedLogin {
            id,
            account: super::encryption::encrypt_struct(login, password)?,
        })
    }
}

/// A stored login is either a account, such as a mnemonic, or a list of multiple accounts where
/// each has an inner id. Future proofed for having private key backed accounts.
#[derive(Serialize, Deserialize, Debug, Zeroize, ZeroizeOnDrop)]
#[serde(untagged)]
pub(crate) enum StoredLogin {
    Mnemonic(MnemonicAccount),
    // PrivateKey(PrivateKeyAccount)
    Multiple(MultipleAccounts),
}

impl StoredLogin {
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

    // Return the login as multiple accounts, and if there is only a single mnemonic backed account,
    // return a set containing only the single account paired with the account id passed as function
    // argument.
    pub(crate) fn unwrap_into_multiple_accounts(self, id: AccountId) -> MultipleAccounts {
        match self {
            StoredLogin::Mnemonic(ref account) => {
                vec![WalletAccount::new(id, account.clone())].into()
            }
            StoredLogin::Multiple(ref accounts) => accounts.clone(),
        }
    }
}

/// Multiple stored accounts, each entry having an id and a data field.
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub(crate) struct MultipleAccounts {
    accounts: Vec<WalletAccount>,
}

impl MultipleAccounts {
    pub(crate) fn new() -> Self {
        MultipleAccounts {
            accounts: Vec::new(),
        }
    }

    pub(crate) fn get_accounts(&self) -> impl Iterator<Item = &WalletAccount> {
        self.accounts.iter()
    }

    pub(crate) fn get_account(&self, id: &AccountId) -> Option<&WalletAccount> {
        self.accounts.iter().find(|account| &account.id == id)
    }

    pub(crate) fn get_account_mut(&mut self, id: &AccountId) -> Option<&mut WalletAccount> {
        self.accounts.iter_mut().find(|account| &account.id == id)
    }

    pub(crate) fn get_account_with_mnemonic(
        &self,
        mnemonic: &bip39::Mnemonic,
    ) -> Option<&WalletAccount> {
        self.get_accounts()
            .find(|account| account.mnemonic() == mnemonic)
    }

    pub(crate) fn inner(&self) -> &[WalletAccount] {
        &self.accounts
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
        mnemonic: Mnemonic,
        hd_path: DerivationPath,
    ) -> Result<(), BackendError> {
        if self.get_account(&id).is_some() {
            Err(BackendError::WalletAccountIdAlreadyExistsInWalletLogin)
        } else if self.get_account_with_mnemonic(&mnemonic).is_some() {
            Err(BackendError::WalletMnemonicAlreadyExistsInWalletLogin)
        } else {
            self.accounts.push(WalletAccount::new(
                id,
                MnemonicAccount::new(mnemonic, hd_path),
            ));
            Ok(())
        }
    }

    pub(crate) fn remove(&mut self, id: &AccountId) -> Result<(), BackendError> {
        if self.get_account(id).is_none() {
            return Err(BackendError::WalletNoSuchAccountIdInWalletLogin);
        }
        self.accounts.retain(|accounts| &accounts.id != id);
        Ok(())
    }

    pub(crate) fn rename(
        &mut self,
        id: &AccountId,
        new_id: &AccountId,
    ) -> Result<(), BackendError> {
        if self.get_account(new_id).is_some() {
            return Err(BackendError::WalletAccountIdAlreadyExistsInWalletLogin);
        }
        let account = self
            .get_account_mut(id)
            .ok_or(BackendError::WalletNoSuchAccountIdInWalletLogin)?;
        account.rename_id(new_id.clone());
        Ok(())
    }
}

impl From<Vec<WalletAccount>> for MultipleAccounts {
    fn from(accounts: Vec<WalletAccount>) -> MultipleAccounts {
        Self { accounts }
    }
}

/// An entry in the list of stored accounts
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub(crate) struct WalletAccount {
    id: AccountId,
    account: AccountData,
}

impl WalletAccount {
    pub(crate) fn new(id: AccountId, mnemonic_account: MnemonicAccount) -> Self {
        Self {
            id,
            account: AccountData::Mnemonic(mnemonic_account),
        }
    }

    pub(crate) fn id(&self) -> &AccountId {
        &self.id
    }

    pub(crate) fn rename_id(&mut self, new_id: AccountId) {
        self.id = new_id;
    }

    pub(crate) fn mnemonic(&self) -> &bip39::Mnemonic {
        match self.account {
            AccountData::Mnemonic(ref account) => account.mnemonic(),
        }
    }

    #[cfg(test)]
    pub(crate) fn hd_path(&self) -> &DerivationPath {
        match self.account {
            AccountData::Mnemonic(ref account) => account.hd_path(),
        }
    }
}

/// An account usually is a mnemonic account, but in the future it might be backed by a private
/// key.
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
#[serde(untagged)]
enum AccountData {
    Mnemonic(MnemonicAccount),
    // PrivateKey(PrivateKeyAccount)
}

/// An account backed by a unique mnemonic.
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop, PartialEq, Eq)]
pub(crate) struct MnemonicAccount {
    mnemonic: bip39::Mnemonic,
    #[serde(with = "display_hd_path")]
    // there's nothing secret about our derivation path
    #[zeroize(skip)]
    hd_path: DerivationPath,
}

impl MnemonicAccount {
    pub(crate) fn new(mnemonic: bip39::Mnemonic, hd_path: DerivationPath) -> Self {
        Self { mnemonic, hd_path }
    }

    pub(crate) fn mnemonic(&self) -> &bip39::Mnemonic {
        &self.mnemonic
    }

    #[cfg(test)]
    pub(crate) fn hd_path(&self) -> &DerivationPath {
        &self.hd_path
    }
}

mod display_hd_path {
    use nym_validator_client::nyxd::bip32::DerivationPath;
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
