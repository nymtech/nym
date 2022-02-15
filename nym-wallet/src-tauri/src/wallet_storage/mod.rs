// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::wallet_storage::account_data::StoredAccount;
use crate::wallet_storage::encryption::{encrypt_struct, EncryptedData};
use crate::wallet_storage::password::UserPassword;
use cosmrs::bip32::DerivationPath;
use std::fs::{create_dir_all, OpenOptions};
use std::path::PathBuf;
pub(crate) mod account_data;
pub(crate) mod encryption;
mod password;

const STORAGE_DIR_NAME: &str = "NymWallet";
const WALLET_INFO_FILENAME: &str = "saved_wallet.json";

fn get_storage_directory() -> Result<PathBuf, BackendError> {
  tauri::api::path::local_data_dir()
    .map(|dir| dir.join(STORAGE_DIR_NAME))
    .ok_or(BackendError::UnknownStorageDirectory)
}

pub(crate) fn wallet_login_filepath() -> Result<PathBuf, BackendError> {
  get_storage_directory().map(|dir| dir.join(WALLET_INFO_FILENAME))
}

pub(crate) fn load_existing_wallet_login_information(
  password: &UserPassword,
) -> Result<Vec<StoredAccount>, BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);

  load_existing_wallet_login_information_at_file(filepath, password)
}

fn load_existing_wallet_login_information_at_file(
  filepath: PathBuf,
  password: &UserPassword,
) -> Result<Vec<StoredAccount>, BackendError> {
  if !filepath.exists() {
    return Ok(Vec::new());
  }
  let file = OpenOptions::new().read(true).open(filepath)?;
  let encrypted_data: EncryptedData<Vec<StoredAccount>> = serde_json::from_reader(file)?;
  encrypted_data.decrypt_struct(password)
}

pub(crate) fn store_wallet_login_information(
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  password: UserPassword,
) -> Result<(), BackendError> {
  // make sure the entire directory structure exists
  let store_dir = get_storage_directory()?;
  create_dir_all(&store_dir)?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);

  store_wallet_login_information_at_file(filepath, mnemonic, hd_path, &password)
}

fn store_wallet_login_information_at_file(
  filepath: PathBuf,
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  password: &UserPassword,
) -> Result<(), BackendError> {
  let mut all_accounts =
    load_existing_wallet_login_information_at_file(filepath.clone(), password)?;
  let new_account = StoredAccount::new_mnemonic_backed_account(mnemonic, hd_path);
  all_accounts.push(new_account);

  let encrypted = encrypt_struct(&all_accounts, password)?;

  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filepath)?;

  serde_json::to_writer_pretty(file, &encrypted)?;

  Ok(())
}

// this function should probably exist, but I guess we need to discuss how it should behave in the context of the UX
// pub(crate) fn remove_wallet_login_information(
//
// )

#[cfg(test)]
mod tests {
  use super::*;
  use config::defaults::COSMOS_DERIVATION_PATH;
  use std::path::Path;
  use tempfile::tempdir;

  fn read_encrypted_blob(file: PathBuf) -> EncryptedData<Vec<StoredAccount>> {
    let file = OpenOptions::new().read(true).open(&file).unwrap();
    serde_json::from_reader(file).unwrap()
  }

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

    // nothing was stored on the disk, so regardless of password used, there will be no error, but
    // returned list will be empty
    assert!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &password)
        .unwrap()
        .is_empty()
    );
    assert!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &bad_password)
        .unwrap()
        .is_empty()
    );

    // store the first account
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      &password,
    )
    .unwrap();

    let encrypted_blob = read_encrypted_blob(wallet_file.clone());

    // some actual ciphertext was saved
    assert!(!encrypted_blob.ciphertext().is_empty());

    // keep track of salt and iv for future assertion
    let original_iv = encrypted_blob.iv().to_vec();
    let original_salt = encrypted_blob.salt().to_vec();

    // trying to load it with wrong password now fails
    assert!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &bad_password).is_err()
    );

    let loaded_accounts =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &password).unwrap();
    println!("{:?}", loaded_accounts);
    assert_eq!(1, loaded_accounts.len());

    let StoredAccount::Mnemonic(acc) = &loaded_accounts[0];
    assert_eq!(&dummy_account1, acc.mnemonic());
    assert_eq!(&cosmos_hd_path, acc.hd_path());

    // can't store extra account if you use different password
    assert!(store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      cosmos_hd_path.clone(),
      &bad_password,
    )
    .is_err());

    // add extra account properly now
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      different_hd_path.clone(),
      &password,
    )
    .unwrap();

    let encrypted_blob = read_encrypted_blob(wallet_file.clone());

    // fresh IV and salt are used
    assert_ne!(original_iv, encrypted_blob.iv());
    assert_ne!(original_salt, encrypted_blob.salt());

    let loaded_accounts =
      load_existing_wallet_login_information_at_file(wallet_file, &password).unwrap();
    assert_eq!(2, loaded_accounts.len());

    // first account should be unchanged
    let StoredAccount::Mnemonic(acc1) = &loaded_accounts[0];
    assert_eq!(&dummy_account1, acc1.mnemonic());
    assert_eq!(&cosmos_hd_path, acc1.hd_path());

    let StoredAccount::Mnemonic(acc2) = &loaded_accounts[1];
    assert_eq!(&dummy_account2, acc2.mnemonic());
    assert_eq!(&different_hd_path, acc2.hd_path());
  }
}
