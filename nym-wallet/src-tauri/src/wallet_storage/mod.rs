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

pub(crate) fn load_existing_wallet_login_information(
  password: &UserPassword,
) -> Result<Vec<StoredAccount>, BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);

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

  let mut all_accounts = load_existing_wallet_login_information(&password)?;
  let new_account = StoredAccount::new_mnemonic_backed_account(mnemonic, hd_path);
  all_accounts.push(new_account);

  let encrypted = encrypt_struct(&all_accounts, &password)?;

  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filepath)?;

  serde_json::to_writer_pretty(file, &encrypted)?;

  // as the function exits, password will be dropped (and zeroed) and mnemonic will be overwritten with a fresh one
  Ok(())
}
