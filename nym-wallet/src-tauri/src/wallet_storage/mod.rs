// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) use crate::wallet_storage::account_data::{StoredLogin, WalletAccount};
pub(crate) use crate::wallet_storage::password::{AccountId, UserPassword};

use crate::error::BackendError;
use crate::platform_constants::{STORAGE_DIR_NAME, WALLET_INFO_FILENAME};
use crate::wallet_storage::encryption::encrypt_struct;
use cosmrs::bip32::DerivationPath;
use std::fs::{self, create_dir_all, OpenOptions};
use std::path::PathBuf;

use self::account_data::{EncryptedLogin, StoredWallet};

pub(crate) mod account_data;
pub(crate) mod encryption;

mod password;

pub(crate) const DEFAULT_WALLET_ACCOUNT_ID: &str = "default";

fn get_storage_directory() -> Result<PathBuf, BackendError> {
  tauri::api::path::local_data_dir()
    .map(|dir| dir.join(STORAGE_DIR_NAME))
    .ok_or(BackendError::UnknownStorageDirectory)
}

pub(crate) fn wallet_login_filepath() -> Result<PathBuf, BackendError> {
  get_storage_directory().map(|dir| dir.join(WALLET_INFO_FILENAME))
}

/// Load stored wallet file
#[allow(unused)]
pub(crate) fn load_existing_wallet() -> Result<StoredWallet, BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  load_existing_wallet_at_file(filepath)
}

fn load_existing_wallet_at_file(filepath: PathBuf) -> Result<StoredWallet, BackendError> {
  if !filepath.exists() {
    return Err(BackendError::WalletFileNotFound);
  }
  let file = OpenOptions::new().read(true).open(filepath)?;
  let wallet: StoredWallet = serde_json::from_reader(file)?;
  Ok(wallet)
}

/// Load the stored wallet file and return the stored login for the given id.
/// The returned login is either an account or list of (inner id, account) pairs.
pub(crate) fn load_existing_wallet_login_information(
  id: &AccountId,
  password: &UserPassword,
) -> Result<StoredLogin, BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  load_existing_wallet_login_information_at_file(filepath, id, password)
}

pub(crate) fn load_existing_wallet_login_information_at_file(
  filepath: PathBuf,
  id: &AccountId,
  password: &UserPassword,
) -> Result<StoredLogin, BackendError> {
  load_existing_wallet_at_file(filepath)?.decrypt_login(id, password)
}

/// Encrypt `mnemonic` and store it together with `id`. It is stored at the top-level.
/// Currently we enforce that we can only add entries with the same password as the other already
/// existing entries. This is not unlikely to change in the future.
pub(crate) fn store_wallet_login_information(
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  id: AccountId,
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
  id: AccountId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  let mut stored_wallet = match load_existing_wallet_at_file(filepath.clone()) {
    Err(BackendError::WalletFileNotFound) => StoredWallet::default(),
    result => result?,
  };

  // Confirm that the given password also can unlock the other entries
  if !stored_wallet.password_can_decrypt_all(password) {
    return Err(BackendError::WalletDifferentPasswordDetected);
  }

  let new_account = StoredLogin::new_mnemonic_backed_account(mnemonic, hd_path);
  let new_encrypted_account = EncryptedLogin {
    id,
    account: encrypt_struct(&new_account, password)?,
  };

  stored_wallet.add_encrypted_login(new_encrypted_account)?;

  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filepath)?;

  Ok(serde_json::to_writer_pretty(file, &stored_wallet)?)
}

/// Append an account to an already existing top-level encrypted account entry.
/// If the existing top-level entry is just a single account, it will be converted to the first
/// account in the list of accounts associated with the encrypted entry. The inner id for this
/// entry will be set to the same as the outer, unencrypted, id.
pub(crate) fn append_account_to_wallet_login_information(
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  id: AccountId,
  inner_id: AccountId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  // make sure the entire directory structure exists
  let store_dir = get_storage_directory()?;
  create_dir_all(&store_dir)?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);

  append_account_to_wallet_login_information_at_file(
    filepath, mnemonic, hd_path, id, inner_id, password,
  )
}

fn append_account_to_wallet_login_information_at_file(
  filepath: PathBuf,
  mnemonic: bip39::Mnemonic,
  hd_path: DerivationPath,
  id: AccountId,
  inner_id: AccountId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  let mut stored_wallet = match load_existing_wallet_at_file(filepath.clone()) {
    Err(BackendError::WalletFileNotFound) => StoredWallet::default(),
    result => result?,
  };

  let decrypted_login = stored_wallet.decrypt_login(&id, password)?;

  // Add accounts to the inner structure
  let mut accounts = decrypted_login.unwrap_into_multiple_accounts(id.clone());
  accounts.add(inner_id, mnemonic, hd_path)?;

  let encrypted_accounts = EncryptedLogin {
    id,
    account: encrypt_struct(&StoredLogin::Multiple(accounts), password)?,
  };

  stored_wallet.replace_encrypted_login(encrypted_accounts)?;

  let file = OpenOptions::new()
    .create(true)
    .write(true)
    .truncate(true)
    .open(filepath)?;

  Ok(serde_json::to_writer_pretty(file, &stored_wallet)?)
}

/// Remove the entire encrypted login entry for the given `id`. This means potentially removing all
/// associated accounts!
/// If this was the last entry in the file, the file is removed.
pub(crate) fn remove_wallet_login_information(id: &AccountId) -> Result<(), BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  remove_wallet_login_information_at_file(filepath, id)
}

fn remove_wallet_login_information_at_file(
  filepath: PathBuf,
  id: &AccountId,
) -> Result<(), BackendError> {
  log::warn!("Removing wallet account with id: {id}. This includes all associated accounts!");
  let mut stored_wallet = match load_existing_wallet_at_file(filepath.clone()) {
    Err(BackendError::WalletFileNotFound) => StoredWallet::default(),
    result => result?,
  };

  if stored_wallet.is_empty() {
    log::info!("Removing file: {:#?}", filepath);
    return Ok(fs::remove_file(filepath)?);
  }

  stored_wallet
    .remove_encrypted_login(id)
    .ok_or(BackendError::NoSuchIdInWallet)?;

  if stored_wallet.is_empty() {
    log::info!("Removing file: {:#?}", filepath);
    Ok(fs::remove_file(filepath)?)
  } else {
    let file = OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open(filepath)?;

    Ok(serde_json::to_writer_pretty(file, &stored_wallet)?)
  }
}

/// Remove an account from inside the encrypted login.
/// - If the encrypted login is just a single account, abort to be on the safe side.
/// - If it is the last associated account with that login, the encrypted login will be removed.
/// - If this was the last encrypted login in the file, it will be removed.
pub(crate) fn remove_account_from_wallet_login(
  id: &AccountId,
  inner_id: &AccountId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  let store_dir = get_storage_directory()?;
  let filepath = store_dir.join(WALLET_INFO_FILENAME);
  remove_account_from_wallet_login_at_file(filepath, id, inner_id, password)
}

fn remove_account_from_wallet_login_at_file(
  filepath: PathBuf,
  id: &AccountId,
  inner_id: &AccountId,
  password: &UserPassword,
) -> Result<(), BackendError> {
  log::info!("Removing associated account from login account: {id}");
  let mut stored_wallet = match load_existing_wallet_at_file(filepath.clone()) {
    Err(BackendError::WalletFileNotFound) => StoredWallet::default(),
    result => result?,
  };

  let mut decrypted_login = stored_wallet.decrypt_login(id, password)?;

  let is_empty = match decrypted_login {
    StoredLogin::Mnemonic(_) => {
      log::warn!("Encountered mnemonic login instead of list of accounts, aborting");
      return Err(BackendError::WalletUnexpectedMnemonicAccount);
    }
    StoredLogin::Multiple(ref mut accounts) => {
      accounts.remove(inner_id)?;
      accounts.is_empty()
    }
  };

  if is_empty {
    stored_wallet
      .remove_encrypted_login(id)
      .ok_or(BackendError::NoSuchIdInWallet)?;
  } else {
    // Replace the encrypted login with the pruned one.
    let encrypted_accounts = EncryptedLogin {
      id: id.clone(),
      account: encrypt_struct(&decrypted_login, password)?,
    };
    stored_wallet.replace_encrypted_login(encrypted_accounts)?;
  }

  if stored_wallet.is_empty() {
    log::info!("Removing file: {:#?}", filepath);
    Ok(fs::remove_file(filepath)?)
  } else {
    let file = OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open(filepath)?;

    Ok(serde_json::to_writer_pretty(file, &stored_wallet)?)
  }
}

#[cfg(test)]
mod tests {
  use crate::wallet_storage::WalletAccount;

  use super::*;
  use config::defaults::COSMOS_DERIVATION_PATH;
  use std::str::FromStr;
  use tempfile::tempdir;

  #[test]
  fn trying_to_load_nonexistant_wallet_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
    let id1 = AccountId::new("first".to_string());
    let password = UserPassword::new("password".to_string());

    assert!(matches!(
      load_existing_wallet_at_file(wallet_file.clone()),
      Err(BackendError::WalletFileNotFound),
    ));
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password),
      Err(BackendError::WalletFileNotFound),
    ));
    remove_wallet_login_information_at_file(wallet_file, &id1).unwrap_err();
  }

  #[test]
  fn store_single_login() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let password = UserPassword::new("password".to_string());
    let id1 = AccountId::new("first".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path,
      id1,
      &password,
    )
    .unwrap();

    let stored_wallet = load_existing_wallet_at_file(wallet_file).unwrap();
    assert_eq!(stored_wallet.len(), 1);

    let login = stored_wallet.get_encrypted_login_by_index(0).unwrap();
    assert_eq!(login.id, AccountId::new("first".to_string()));

    // some actual ciphertext was saved
    assert!(!login.account.ciphertext().is_empty());
  }

  #[test]
  fn store_twice_for_the_same_id_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let password = UserPassword::new("password".to_string());
    let id1 = AccountId::new("first".to_string());

    // Store the first login
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    // and storing the same id again fails
    assert!(matches!(
      store_wallet_login_information_at_file(
        wallet_file,
        dummy_account1,
        cosmos_hd_path,
        id1,
        &password,
      ),
      Err(BackendError::IdAlreadyExistsInWallet),
    ));
  }

  #[test]
  fn load_with_wrong_password_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let password = UserPassword::new("password".to_string());
    let bad_password = UserPassword::new("bad-password".to_string());
    let id1 = AccountId::new("first".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path,
      id1.clone(),
      &password,
    )
    .unwrap();

    // Trying to load it with wrong password now fails
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file, &id1, &bad_password),
      Err(BackendError::DecryptionError),
    ));
  }

  #[test]
  fn load_with_wrong_id_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let password = UserPassword::new("password".to_string());
    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path,
      id1,
      &password,
    )
    .unwrap();

    // Trying to load with the wrong id
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file, &id2, &password),
      Err(BackendError::NoSuchIdInWallet),
    ));
  }

  #[test]
  fn store_and_load_a_single_login() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let password = UserPassword::new("password".to_string());
    let id1 = AccountId::new("first".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file, &id1, &password).unwrap();
    let acc = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account1, acc.mnemonic());
    assert_eq!(&cosmos_hd_path, acc.hd_path());
  }

  #[test]
  fn store_a_second_login_with_a_different_password_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());
    let bad_password = UserPassword::new("bad-password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path.clone(),
      id1,
      &password,
    )
    .unwrap();

    // Can't store a second login if you use different password
    assert!(matches!(
      store_wallet_login_information_at_file(
        wallet_file,
        dummy_account2,
        cosmos_hd_path,
        id2,
        &bad_password
      ),
      Err(BackendError::WalletDifferentPasswordDetected),
    ));
  }

  #[test]
  fn store_two_mnemonic_accounts_gives_different_salts_and_iv() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let different_hd_path: DerivationPath = "m".parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    // Store the first account
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path,
      id1,
      &password,
    )
    .unwrap();

    let stored_wallet = load_existing_wallet_at_file(wallet_file.clone()).unwrap();
    let encrypted_blob = &stored_wallet
      .get_encrypted_login_by_index(0)
      .unwrap()
      .account;

    // keep track of salt and iv for future assertion
    let original_iv = encrypted_blob.iv().to_vec();
    let original_salt = encrypted_blob.salt().to_vec();

    // Add an extra account
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2,
      different_hd_path,
      id2,
      &password,
    )
    .unwrap();

    let loaded_accounts = load_existing_wallet_at_file(wallet_file).unwrap();
    assert_eq!(loaded_accounts.len(), 2);
    let encrypted_blob = &loaded_accounts
      .get_encrypted_login_by_index(1)
      .unwrap()
      .account;

    // fresh IV and salt are used
    assert_ne!(original_iv, encrypted_blob.iv());
    assert_ne!(original_salt, encrypted_blob.salt());
  }

  #[test]
  fn store_two_mnemonic_accounts_using_two_logins() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let different_hd_path: DerivationPath = "m".parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    // Store the first account
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let acc = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account1, acc.mnemonic());
    assert_eq!(&cosmos_hd_path, acc.hd_path());

    // Add an extra account
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      different_hd_path.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    // first account should be unchanged
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let acc1 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account1, acc1.mnemonic());
    assert_eq!(&cosmos_hd_path, acc1.hd_path());

    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file, &id2, &password).unwrap();
    let acc2 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account2, acc2.mnemonic());
    assert_eq!(&different_hd_path, acc2.hd_path());
  }

  #[test]
  fn remove_non_existent_id_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path,
      id1,
      &password,
    )
    .unwrap();

    // Fails to delete non-existent id in the wallet
    assert!(matches!(
      remove_wallet_login_information_at_file(wallet_file, &id2),
      Err(BackendError::NoSuchIdInWallet),
    ));
  }

  #[test]
  fn store_and_remove_wallet_login_information() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let different_hd_path: DerivationPath = "m".parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    // Store two accounts with two different passwords
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();
    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      different_hd_path.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    // Load and compare
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let acc1 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account1, acc1.mnemonic());
    assert_eq!(&cosmos_hd_path, acc1.hd_path());

    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id2, &password).unwrap();
    let acc2 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account2, acc2.mnemonic());
    assert_eq!(&different_hd_path, acc2.hd_path());

    // Delete the second account
    remove_wallet_login_information_at_file(wallet_file.clone(), &id2).unwrap();

    // The first account should be unchanged
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let acc1 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(&dummy_account1, acc1.mnemonic());
    assert_eq!(&cosmos_hd_path, acc1.hd_path());

    // And we can't load the second one anymore
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id2, &password),
      Err(BackendError::NoSuchIdInWallet),
    ));

    // Delete the first account
    assert!(wallet_file.exists());
    remove_wallet_login_information_at_file(wallet_file.clone(), &id1).unwrap();

    // The file should now be removed
    assert!(!wallet_file.exists());

    // And trying to load when the file is gone fails
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file, &id1, &password),
      Err(BackendError::WalletFileNotFound),
    ));
  }

  #[test]
  fn append_account_converts_the_type() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    // Check that it's there as the correct non-multiple type
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let acc1 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(acc1.mnemonic(), &dummy_account1);
    assert_eq!(acc1.hd_path(), &cosmos_hd_path);

    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    // Check that it is now multiple mnemonic type
    let loaded_accounts =
      load_existing_wallet_login_information_at_file(wallet_file, &id1, &password).unwrap();
    let accounts = loaded_accounts.as_multiple_accounts().unwrap();
    let expected = vec![
      WalletAccount::new_mnemonic_backed_account(id1, dummy_account1, cosmos_hd_path.clone()),
      WalletAccount::new_mnemonic_backed_account(id2, dummy_account2, cosmos_hd_path),
    ]
    .into();
    assert_eq!(accounts, &expected);
  }

  #[test]
  fn append_accounts_to_existing_login() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account3 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account4 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());
    let id3 = AccountId::new("third".to_string());
    let id4 = AccountId::new("fourth".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    // Check that it's there as the correct non-multiple type
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id2, &password).unwrap();
    let acc2 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(acc2.mnemonic(), &dummy_account2);
    assert_eq!(acc2.hd_path(), &cosmos_hd_path);

    // Add a third and fourth mnenonic grouped together with the second one
    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account3.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      id3.clone(),
      &password,
    )
    .unwrap();
    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account4.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      id4.clone(),
      &password,
    )
    .unwrap();

    // Check that we can load all four
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password).unwrap();
    let acc1 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(acc1.mnemonic(), &dummy_account1);
    assert_eq!(acc1.hd_path(), &cosmos_hd_path);

    let loaded_accounts =
      load_existing_wallet_login_information_at_file(wallet_file, &id2, &password).unwrap();
    let accounts = loaded_accounts.as_multiple_accounts().unwrap();
    let expected = vec![
      WalletAccount::new_mnemonic_backed_account(id2, dummy_account2, cosmos_hd_path.clone()),
      WalletAccount::new_mnemonic_backed_account(id3, dummy_account3, cosmos_hd_path.clone()),
      WalletAccount::new_mnemonic_backed_account(id4, dummy_account4, cosmos_hd_path),
    ]
    .into();
    assert_eq!(accounts, &expected);
  }

  #[test]
  fn delete_the_same_account_twice_for_a_login_fails() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2,
      cosmos_hd_path,
      id1.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id1, &id2, &password).unwrap();

    assert!(matches!(
      remove_account_from_wallet_login_at_file(wallet_file, &id1, &id2, &password),
      Err(BackendError::NoSuchIdInWalletLoginEntry),
    ));
  }

  #[test]
  fn delete_appended_account_doesnt_affect_others() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account3 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());
    let id3 = AccountId::new("third".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account3.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      id3.clone(),
      &password,
    )
    .unwrap();

    remove_wallet_login_information_at_file(wallet_file.clone(), &id1).unwrap();

    // The second login one is still there
    let loaded_accounts =
      load_existing_wallet_login_information_at_file(wallet_file, &id2, &password).unwrap();
    let accounts = loaded_accounts.as_multiple_accounts().unwrap();
    let expected = vec![
      WalletAccount::new_mnemonic_backed_account(id2, dummy_account2, cosmos_hd_path.clone()),
      WalletAccount::new_mnemonic_backed_account(id3, dummy_account3, cosmos_hd_path),
    ]
    .into();
    assert_eq!(accounts, &expected);
  }

  #[test]
  fn remove_all_accounts_for_a_login_removes_the_file_when_empty() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2,
      cosmos_hd_path,
      id1.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id1, &id1, &password).unwrap();
    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id1, &id2, &password).unwrap();

    // The file should now be removed
    assert!(!wallet_file.exists());

    // And trying to load when the file is gone fails
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file, &id1, &password),
      Err(BackendError::WalletFileNotFound),
    ));
  }

  #[test]
  fn remove_all_accounts_for_a_login_removes_that_login() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account3 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());
    let id3 = AccountId::new("third".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1,
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2,
      cosmos_hd_path.clone(),
      id1.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account3.clone(),
      cosmos_hd_path.clone(),
      id3.clone(),
      &password,
    )
    .unwrap();

    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id1, &id1, &password).unwrap();
    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id1, &id2, &password).unwrap();

    // And trying to load when the file is gone fails
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id1, &password),
      Err(BackendError::NoSuchIdInWallet),
    ));

    // The other login is still there
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file, &id3, &password).unwrap();
    let acc3 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(acc3.mnemonic(), &dummy_account3);
    assert_eq!(acc3.hd_path(), &cosmos_hd_path);
  }

  #[test]
  fn append_accounts_and_remove_appended_accounts() {
    let store_dir = tempdir().unwrap();
    let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);

    let dummy_account1 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account2 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account3 = bip39::Mnemonic::generate(24).unwrap();
    let dummy_account4 = bip39::Mnemonic::generate(24).unwrap();
    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let password = UserPassword::new("password".to_string());

    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());
    let id3 = AccountId::new("third".to_string());
    let id4 = AccountId::new("fourth".to_string());

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account1.clone(),
      cosmos_hd_path.clone(),
      id1.clone(),
      &password,
    )
    .unwrap();

    store_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account2.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      &password,
    )
    .unwrap();

    // Add a third and fourth mnenonic grouped together with the second one
    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account3,
      cosmos_hd_path.clone(),
      id2.clone(),
      id3.clone(),
      &password,
    )
    .unwrap();
    append_account_to_wallet_login_information_at_file(
      wallet_file.clone(),
      dummy_account4.clone(),
      cosmos_hd_path.clone(),
      id2.clone(),
      id4.clone(),
      &password,
    )
    .unwrap();

    // Delete the third mnemonic, from the second login entry
    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id2, &id3, &password).unwrap();

    // Check that we can still load the other accounts
    let loaded_accounts =
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id2, &password).unwrap();
    let accounts = loaded_accounts.as_multiple_accounts().unwrap();
    let expected = vec![
      WalletAccount::new_mnemonic_backed_account(
        id2.clone(),
        dummy_account2,
        cosmos_hd_path.clone(),
      ),
      WalletAccount::new_mnemonic_backed_account(
        id4.clone(),
        dummy_account4,
        cosmos_hd_path.clone(),
      ),
    ]
    .into();
    assert_eq!(accounts, &expected);

    // Delete the second and fourth mnemonic from the second login entry removes the login entry
    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id2, &id2, &password).unwrap();
    remove_account_from_wallet_login_at_file(wallet_file.clone(), &id2, &id4, &password).unwrap();
    assert!(matches!(
      load_existing_wallet_login_information_at_file(wallet_file.clone(), &id2, &password),
      Err(BackendError::NoSuchIdInWallet),
    ));

    // The first login is still available
    let loaded_account =
      load_existing_wallet_login_information_at_file(wallet_file, &id1, &password).unwrap();
    let acc1 = loaded_account.as_mnemonic_account().unwrap();
    assert_eq!(acc1.mnemonic(), &dummy_account1);
    assert_eq!(acc1.hd_path(), &cosmos_hd_path);
  }

  // Test to that decrypts a stored file from the repo, to make sure we are able to decrypt stored
  // wallets created with older versions.
  #[test]
  fn decrypt_stored_wallet() {
    const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet.json";
    let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);

    let wallet = load_existing_wallet_at_file(wallet_file).unwrap();

    let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
    let password = UserPassword::new("password".to_string());
    let bad_password = UserPassword::new("bad-password".to_string());
    let id1 = AccountId::new("first".to_string());
    let id2 = AccountId::new("second".to_string());

    assert!(!wallet.password_can_decrypt_all(&bad_password));
    assert!(wallet.password_can_decrypt_all(&password));

    let account1 = wallet.decrypt_login(&id1, &password).unwrap();
    let account2 = wallet.decrypt_login(&id2, &password).unwrap();

    assert!(matches!(account1, StoredLogin::Mnemonic(_)));
    assert!(matches!(account2, StoredLogin::Mnemonic(_)));

    let expected_account1 = bip39::Mnemonic::from_str("country mean universe text phone begin deputy reject result good cram illness common cluster proud swamp digital patrol spread bar face december base kick").unwrap();
    let expected_account2 =  bip39::Mnemonic::from_str("home mansion start quiz dress decide hint second dragon sunny juice always steak real minimum art rival skin draw total pulp foot goddess agent").unwrap();

    assert_eq!(
      account1.as_mnemonic_account().unwrap().mnemonic(),
      &expected_account1
    );
    assert_eq!(
      account1.as_mnemonic_account().unwrap().hd_path(),
      &cosmos_hd_path,
    );

    assert_eq!(
      account2.as_mnemonic_account().unwrap().mnemonic(),
      &expected_account2
    );
    assert_eq!(
      account2.as_mnemonic_account().unwrap().hd_path(),
      &cosmos_hd_path,
    );
  }

  #[test]
  fn decrypt_stored_wallet_with_multiple_accounts() {
    // WIP(JON)
  }
}
