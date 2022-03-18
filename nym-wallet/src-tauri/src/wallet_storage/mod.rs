// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::operations::mixnet::account::create_new_account;
pub(crate) use crate::wallet_storage::password::{UserId, UserPassword};

use crate::error::BackendError;
use crate::platform_constants::{STORAGE_DIR_NAME, WALLET_INFO_FILENAME};
use crate::wallet_storage::account_data::StoredAccount;
use crate::wallet_storage::encryption::{encrypt_struct, EncryptedData};
use cosmrs::bip32::DerivationPath;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, OpenOptions};
use std::path::PathBuf;

use self::account_data::{EncryptedAccount, StoredWallet};

pub(crate) mod account_data;
pub(crate) mod encryption;

mod password;

fn get_storage_directory() -> Result<PathBuf, BackendError> {
  tauri::api::path::local_data_dir()
    .map(|dir| dir.join(STORAGE_DIR_NAME))
    .ok_or(BackendError::UnknownStorageDirectory)
}

pub(crate) fn wallet_login_filepath() -> Result<PathBuf, BackendError> {
  get_storage_directory().map(|dir| dir.join(WALLET_INFO_FILENAME))
}

pub(crate) fn load_existing_wallet(password: &UserPassword) -> Result<StoredWallet, BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  load_existing_wallet_at_file(filepath)
}

fn load_existing_wallet_at_file(filepath: PathBuf) -> Result<StoredWallet, BackendError> {
  if !filepath.exists() {
    return Err(BackendError::WalletNotFound);
  }
  let file = OpenOptions::new().read(true).open(filepath)?;
  let wallet: StoredWallet = serde_json::from_reader(file)?;
  Ok(wallet)
}

pub(crate) fn load_existing_wallet_login_information(
  id: &UserId,
  password: &UserPassword,
) -> Result<StoredAccount, BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  load_existing_wallet_login_information_at_file(filepath, id, password)
}

fn load_existing_wallet_login_information_at_file(
  filepath: PathBuf,
  id: &UserId,
  password: &UserPassword,
) -> Result<StoredAccount, BackendError> {
  load_existing_wallet_at_file(filepath)?.decrypt_account(id, password)
}

pub(crate) fn store_wallet_login_information(
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  id: UserId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  // make sure the entire directory structure exists
  let store_dir = get_storage_directory()?;
  create_dir_all(&store_dir)?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);

  store_wallet_login_information_at_file(filepath, mnemonic, hd_path, id, password)
}

fn store_wallet_login_information_at_file(
  filepath: PathBuf,
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  id: UserId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  let mut stored_wallet = match load_existing_wallet_at_file(filepath.clone()) {
    Err(BackendError::WalletNotFound) => StoredWallet::default(),
    result => result?,
  };

  // Confirm that the given password also can unlock the other entries
  if !stored_wallet.password_can_decrypt_all(password) {
    return Err(BackendError::WalletDifferentPasswordDetected);
  }

  let new_account = StoredAccount::new_mnemonic_backed_account(mnemonic, hd_path);
  let new_encrypted_account = EncryptedAccount {
    id,
    account: encrypt_struct(&new_account, password)?,
  };

  stored_wallet.accounts.push(new_encrypted_account);

  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filepath)?;

  Ok(serde_json::to_writer_pretty(file, &stored_wallet)?)
}

// this function should probably exist, but I guess we need to discuss how it should behave in the context of the UX
// pub(crate) fn remove_wallet_login_information(
//
// )

#[cfg(test)]
mod tests {
  use crate::wallet_storage::encryption::encrypt_data;

  use super::*;
  use config::defaults::COSMOS_DERIVATION_PATH;
  use std::path::Path;
  use tempfile::tempdir;

  // I'm not 100% sure how to feel about having to touch the file system at all
  #[test]
  fn storing_wallet_information() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let different_hd_path: DerivationPath = "m".parse().unwrap();

    let password = UserPassword::new("password".to_string());
    let bad_password = UserPassword::new("bad-password".to_string());

    let id1 = UserId::new("first".to_string());
    let id2 = UserId::new("second".to_string());

    // Nothing was stored on the disk
    assert!(matches!(
      load_existing_wallet_at_file(wallet_file.clone()),
      Err(BackendError::WalletNotFound),
    ));
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password),
      Err(BackendError::WalletNotFound),
    ));

    // Store the first account
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    let stored_wallet = load_existing_wallet_at_file(wallet_file.clone()).unwrap();
    assert_eq!(stored_wallet.len(), 1);
    assert_eq!(
      stored_wallet.accounts[0].id,
      UserId::new("first".to_string())
    );
    let encrypted_blob = &stored_wallet.accounts[0].account;

    // some actual ciphertext was saved
    assert!(!encrypted_blob.ciphertext().is_empty());

    // keep track of salt and iv for future assertion
    let original_iv = encrypted_blob.iv().to_vec();
    let original_salt = encrypted_blob.salt().to_vec();

    // trying to load it with wrong password now fails
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &bad_password),
      Err(BackendError::DecryptionError),
    ));
    // and with the wrong id also fails
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id2, &password),
      Err(BackendError::NoSuchWalletId),
    ));

    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();

    let StoredAccount::Mnemonic(ref acc) = loaded_account;
    assert_eq!(&dummy_account1, acc.mnemonic());
    assert_eq!(&cosmos_hd_path, acc.hd_path());

    // Can't store extra account if you use different password
    assert!(matches!(
      store_wallet_login_information_at_file(
        wallet_file.clone(),
        dummy_account2.clone(),
        cosmos_hd_path.clone(),
        id2.clone(),
        &bad_password
      ),
      Err(BackendError::WalletDifferentPasswordDetected),
    ));

    // add extra account properly now
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      different_hd_path.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    let loaded_accounts = load_existing_wallet_at_file(wallet_file.clone()).unwrap();
    assert_eq!(2, loaded_accounts.len());
    let encrypted_blob = &loaded_accounts.accounts[1].account;

    // fresh IV and salt are used
    assert_ne!(original_iv, encrypted_blob.iv());
    assert_ne!(original_salt, encrypted_blob.salt());

    // WIP(JON): test that a re-saved account has new IV and salt

    // first account should be unchanged
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let StoredAccount::Mnemonic(ref acc1) = loaded_account;
    assert_eq!(&dummy_account1, acc1.mnemonic());
    assert_eq!(&cosmos_hd_path, acc1.hd_path());

    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file, &id2, &password).unwrap();
    let StoredAccount::Mnemonic(ref acc2) = loaded_account;
    assert_eq!(&dummy_account2, acc2.mnemonic());
    assert_eq!(&different_hd_path, acc2.hd_path());
  }
}
