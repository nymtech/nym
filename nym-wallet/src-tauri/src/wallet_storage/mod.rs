// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// The wallet storage file contains a set of logins, each with an associated login ID and an
/// encrypted field. Once decrypted, each login contains either an account, or a set of accounts.
/// One difference is that the latter have an associated account ID.
///
/// Wallet
/// - Login
///   -- Account
///     --- Mnemonic
pub(crate) use crate::wallet_storage::account_data::StoredLogin;
pub(crate) use crate::wallet_storage::password::{AccountId, LoginId, UserPassword};

use crate::error::BackendError;
use crate::platform_constants::{STORAGE_DIR_NAME, WALLET_INFO_FILENAME};
use bip39::Mnemonic;
use nym_validator_client::nyxd::bip32::DerivationPath;
use std::ffi::OsString;
use std::fs::{self, create_dir_all, OpenOptions};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

#[cfg(test)]
use self::account_data::MnemonicAccount;
use self::account_data::{EncryptedLogin, MultipleAccounts, StoredWallet};

pub(crate) mod account_data;
pub(crate) mod encryption;

mod password;

/// The default wallet (top-level) login id.
pub(crate) const DEFAULT_LOGIN_ID: &str = "default";

/// When converting a single account entry to one that contains many, the first account will use
/// this name.
pub(crate) const DEFAULT_FIRST_ACCOUNT_NAME: &str = "Account 1";

fn get_storage_directory() -> Result<PathBuf, BackendError> {
    tauri::api::path::local_data_dir()
        .map(|dir| dir.join(STORAGE_DIR_NAME))
        .ok_or(BackendError::UnknownStorageDirectory)
}

pub(crate) fn wallet_login_filepath() -> Result<PathBuf, BackendError> {
    get_storage_directory().map(|dir| dir.join(WALLET_INFO_FILENAME))
}

fn write_to_file(filepath: &Path, wallet: &StoredWallet) -> Result<(), BackendError> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(filepath)?;

    Ok(serde_json::to_writer_pretty(file, &wallet)?)
}

/// Load stored wallet file
#[allow(unused)]
pub(crate) fn load_existing_wallet() -> Result<StoredWallet, BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);
    load_existing_wallet_at_file(&filepath)
}

fn load_existing_wallet_at_file(filepath: &Path) -> Result<StoredWallet, BackendError> {
    if !filepath.exists() {
        return Err(BackendError::WalletFileNotFound);
    }
    let file = OpenOptions::new().read(true).open(filepath)?;
    let wallet: StoredWallet = serde_json::from_reader(file)?;
    Ok(wallet)
}

/// Load the stored wallet file and return the stored login for the given id.
/// The returned login is either an account or list of (inner id, account) pairs.
pub(crate) fn load_existing_login(
    id: &LoginId,
    password: &UserPassword,
) -> Result<StoredLogin, BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);
    load_existing_login_at_file(&filepath, id, password)
}

pub(crate) fn load_existing_login_at_file(
    filepath: &Path,
    id: &LoginId,
    password: &UserPassword,
) -> Result<StoredLogin, BackendError> {
    load_existing_wallet_at_file(filepath)?.decrypt_login(id, password)
}

// DEPRECATED: only used in tests, where it's used to test supporting older wallet formats
#[allow(unused)]
#[cfg(test)]
pub(crate) fn store_login(
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    // make sure the entire directory structure exists
    let store_dir = get_storage_directory()?;
    create_dir_all(&store_dir)?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);

    store_login_at_file(&filepath, mnemonic, hd_path, id, password)
}

// DEPRECATED: only used in tests, where it's used to test supporting older wallet formats
#[cfg(test)]
fn store_login_at_file(
    filepath: &Path,
    mnemonic: bip39::Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    let mut stored_wallet = match load_existing_wallet_at_file(filepath) {
        Err(BackendError::WalletFileNotFound) => StoredWallet::default(),
        result => result?,
    };

    // Confirm that the given password also can unlock the other entries.
    // This is restriction we can relax in the future, but for now it's a sanity check.
    if !stored_wallet.password_can_decrypt_all(password) {
        return Err(BackendError::WalletDifferentPasswordDetected);
    }

    let new_account = MnemonicAccount::new(mnemonic, hd_path);
    let new_login = StoredLogin::Mnemonic(new_account);
    let new_encrypted_account = EncryptedLogin::encrypt(id, &new_login, password)?;
    stored_wallet.add_encrypted_login(new_encrypted_account)?;

    write_to_file(filepath, &stored_wallet)
}

/// Store the login with multiple accounts support
pub(crate) fn store_login_with_multiple_accounts(
    mnemonic: Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    // make sure the entire directory structure exists
    let store_dir = get_storage_directory()?;
    create_dir_all(&store_dir)?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);

    store_login_with_multiple_accounts_at_file(&filepath, mnemonic, hd_path, id, password)
}

/// Update all logins with multiple accounts support
pub(crate) fn update_encrypted_logins(
    current_password: &UserPassword,
    new_password: &UserPassword,
) -> Result<(), BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);

    update_encrypted_logins_at_file(&filepath, current_password, new_password)
}

fn update_encrypted_logins_at_file(
    filepath: &Path,
    current_password: &UserPassword,
    new_password: &UserPassword,
) -> Result<(), BackendError> {
    if current_password == new_password {
        return Ok(());
    }
    let mut stored_wallet = load_existing_wallet_at_file(filepath)?;

    stored_wallet.reencrypt_all(current_password, new_password)?;
    write_to_file(filepath, &stored_wallet)
}

fn new_encrypted_login(
    mnemonic: Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    password: &UserPassword,
) -> Result<EncryptedLogin, BackendError> {
    let mut new_accounts = MultipleAccounts::new();
    new_accounts.add(DEFAULT_FIRST_ACCOUNT_NAME.into(), mnemonic, hd_path)?;
    let new_login = StoredLogin::Multiple(new_accounts);
    EncryptedLogin::encrypt(id, &new_login, password)
}

fn store_login_with_multiple_accounts_at_file(
    filepath: &Path,
    mnemonic: Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    let mut stored_wallet = match load_existing_wallet_at_file(filepath) {
        Err(BackendError::WalletFileNotFound) => StoredWallet::default(),
        result => result?,
    };

    // Confirm that the given password also can unlock the other entries.
    // This is restriction we can relax in the future, but for now it's a sanity check.
    if !stored_wallet.password_can_decrypt_all(password) {
        return Err(BackendError::WalletDifferentPasswordDetected);
    }

    let new_login = new_encrypted_login(mnemonic, hd_path, id, password)?;
    stored_wallet.add_encrypted_login(new_login)?;

    write_to_file(filepath, &stored_wallet)
}

/// Append an account to an already existing top-level encrypted account entry.
/// If the existing top-level entry is just a single account, it will be converted to the first
/// account in the list of accounts associated with the encrypted entry. The inner id for this
/// entry will be set to the same as the outer, unencrypted, id.
pub(crate) fn append_account_to_login(
    mnemonic: Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    inner_id: AccountId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    // make sure the entire directory structure exists
    let store_dir = get_storage_directory()?;
    create_dir_all(&store_dir)?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);

    append_account_to_login_at_file(&filepath, mnemonic, hd_path, id, inner_id, password)
}

fn append_account_to_login_at_file(
    filepath: &Path,
    mnemonic: Mnemonic,
    hd_path: DerivationPath,
    id: LoginId,
    inner_id: AccountId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    let mut stored_wallet = load_existing_wallet_at_file(filepath)?;

    let decrypted_login = stored_wallet.decrypt_login(&id, password)?;

    // Add accounts to the inner structure.
    // Note that in case we only have single account entry, without an inner_id, we convert to
    // multiple accounts and we set the first inner_id to id.
    let first_id_when_converting = id.clone().into();
    let mut accounts = decrypted_login.unwrap_into_multiple_accounts(first_id_when_converting);
    accounts.add(inner_id, mnemonic, hd_path)?;

    let encrypted_accounts =
        EncryptedLogin::encrypt(id, &StoredLogin::Multiple(accounts), password)?;

    stored_wallet.replace_encrypted_login(encrypted_accounts)?;

    write_to_file(filepath, &stored_wallet)
}

/// Remove the entire encrypted login entry for the given `id`. This means potentially removing all
/// associated accounts!
/// If this was the last entry in the file, the file is removed.
pub(crate) fn remove_login(id: &LoginId) -> Result<(), BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);
    remove_login_at_file(&filepath, id)
}

fn remove_login_at_file(filepath: &Path, id: &LoginId) -> Result<(), BackendError> {
    log::warn!("Removing wallet account with id: {id}. This includes all associated accounts!");
    let mut stored_wallet = load_existing_wallet_at_file(filepath)?;

    if stored_wallet.is_empty() {
        log::info!("Removing file: {:#?}", filepath);
        return Ok(fs::remove_file(filepath)?);
    }

    stored_wallet
        .remove_encrypted_login(id)
        .ok_or(BackendError::WalletNoSuchLoginId)?;

    if stored_wallet.is_empty() {
        log::info!("Removing file: {:#?}", filepath);
        Ok(fs::remove_file(filepath)?)
    } else {
        write_to_file(filepath, &stored_wallet)
    }
}

// Given a file path, append a timestamp. If provided, also append a number.
fn append_timestamp_to_filename(
    path: impl AsRef<Path>,
    timestamp: OsString,
    additional_number: Option<u32>,
) -> Result<PathBuf, BackendError> {
    let path = path.as_ref();
    let mut result_path = path.to_owned();

    let stem = result_path
        .file_stem()
        .ok_or(BackendError::WalletFileMalformedFilename)?;
    let mut new_stem = stem.to_os_string();
    new_stem.push("-");
    new_stem.push(timestamp);
    if let Some(additional_number) = additional_number {
        new_stem.push("-");
        new_stem.push(additional_number.to_string());
    }
    result_path.set_file_name(new_stem);

    if let Some(ext) = path.extension() {
        result_path.set_extension(ext);
    }
    Ok(result_path)
}

fn _archive_wallet_file(path: &Path) -> Result<(), BackendError> {
    let timestamp: OsString = OffsetDateTime::now_utc()
        .unix_timestamp()
        .to_string()
        .into();
    let mut additional_number = 0;
    let mut new_path = append_timestamp_to_filename(path, timestamp.clone(), None)?;

    // Try rename, and if it fails, try appending a number.
    while additional_number < 10 {
        if fs::rename(path, new_path.clone()).is_err() {
            new_path =
                append_timestamp_to_filename(path, timestamp.clone(), Some(additional_number))?;
            additional_number += 1;
        } else {
            if let Some(new_path) = new_path.to_str() {
                log::info!("Archived to: {}", new_path);
            } else {
                log::warn!("Archived wallet file to filename that is not a valid UTF-8 string");
            }
            return Ok(());
        }
    }

    log::warn!("Failed to archive wallet file, suggest renaming it manually!");
    Err(BackendError::WalletFileUnableToArchive)
}

pub(crate) fn archive_wallet_file() -> Result<(), BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);

    if filepath.exists() {
        if let Some(filepath) = filepath.to_str() {
            log::info!("Archiving wallet file: {}", filepath);
        } else {
            log::info!("Archiving wallet file");
        }
        _archive_wallet_file(&filepath)
    } else {
        if let Some(filepath) = filepath.to_str() {
            log::info!(
                "Skipping archiving wallet file, as it's not found: {}",
                filepath
            );
        } else {
            log::info!("Skipping archiving wallet file, as it's not found");
        }
        Err(BackendError::WalletFileNotFound)
    }
}

/// Remove an account from inside the encrypted login.
/// - If the encrypted login is just a single account, abort to be on the safe side.
/// - If it is the last associated account with that login, the encrypted login will be removed.
/// - If this was the last encrypted login in the file, it will be removed.
pub(crate) fn remove_account_from_login(
    id: &LoginId,
    inner_id: &AccountId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);
    remove_account_from_login_at_file(&filepath, id, inner_id, password)
}

fn remove_account_from_login_at_file(
    filepath: &Path,
    id: &LoginId,
    inner_id: &AccountId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    log::info!("Removing associated account from login account: {id}");
    let mut stored_wallet = load_existing_wallet_at_file(filepath)?;

    let mut decrypted_login = stored_wallet.decrypt_login(id, password)?;

    // Remove the account
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

    // Remove the login, or encrypt the new updated login
    if is_empty {
        stored_wallet
            .remove_encrypted_login(id)
            .ok_or(BackendError::WalletNoSuchLoginId)?;
    } else {
        let encrypted_accounts = EncryptedLogin::encrypt(id.clone(), &decrypted_login, password)?;
        stored_wallet.replace_encrypted_login(encrypted_accounts)?;
    }

    // Remove the file, or write the new file
    if stored_wallet.is_empty() {
        log::info!("Removing file: {:#?}", filepath);
        Ok(fs::remove_file(filepath)?)
    } else {
        write_to_file(filepath, &stored_wallet)
    }
}

/// Rename an account inside the encrypted login.
/// - If the encrypted login is just a single account, abort to be on the safe side.
/// - If the name already exists, abort.
pub(crate) fn rename_account_in_login(
    id: &LoginId,
    account_id: &AccountId,
    new_account_id: &AccountId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    let store_dir = get_storage_directory()?;
    let filepath = store_dir.join(WALLET_INFO_FILENAME);
    rename_account_in_login_at_file(&filepath, id, account_id, new_account_id, password)
}

fn rename_account_in_login_at_file(
    filepath: &Path,
    id: &LoginId,
    account_id: &AccountId,
    new_account_id: &AccountId,
    password: &UserPassword,
) -> Result<(), BackendError> {
    log::info!("Renaming associated account in login account: {id}");
    let mut stored_wallet = load_existing_wallet_at_file(filepath)?;

    let mut decrypted_login = stored_wallet.decrypt_login(id, password)?;

    // Rename the account
    match decrypted_login {
        StoredLogin::Mnemonic(_) => {
            log::warn!("Encountered mnemonic login instead of list of accounts, aborting");
            return Err(BackendError::WalletUnexpectedMnemonicAccount);
        }
        StoredLogin::Multiple(ref mut accounts) => {
            accounts.rename(account_id, new_account_id)?;
        }
    };

    // Encrypt the new updated login and write to file
    let encrypted_accounts = EncryptedLogin::encrypt(id.clone(), &decrypted_login, password)?;
    stored_wallet.replace_encrypted_login(encrypted_accounts)?;
    write_to_file(filepath, &stored_wallet)
}

#[cfg(test)]
mod tests {
    use crate::wallet_storage::account_data::WalletAccount;

    use super::*;
    use nym_config::defaults::COSMOS_DERIVATION_PATH;
    use std::str::FromStr;
    use tempfile::tempdir;

    #[test]
    fn trying_to_load_nonexistant_wallet_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let id1 = LoginId::new("first".to_string());
        let password = UserPassword::new("password".to_string());

        assert!(matches!(
            load_existing_wallet_at_file(&wallet_file),
            Err(BackendError::WalletFileNotFound),
        ));
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id1, &password),
            Err(BackendError::WalletFileNotFound),
        ));
        remove_login_at_file(&wallet_file, &id1).unwrap_err();
    }

    #[test]
    fn store_single_login() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(&wallet_file, account1, hd_path, id1.clone(), &password).unwrap();

        let stored_wallet = load_existing_wallet_at_file(&wallet_file).unwrap();
        assert_eq!(stored_wallet.len(), 1);

        let login = stored_wallet.get_encrypted_login_by_index(0).unwrap();
        assert_eq!(login.id, id1);

        // some actual ciphertext was saved
        assert!(!login.account.ciphertext().is_empty());
    }

    #[test]
    fn store_single_login_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1,
            cosmos_hd_path,
            id1.clone(),
            &password,
        )
        .unwrap();

        let stored_wallet = load_existing_wallet_at_file(&wallet_file).unwrap();
        assert_eq!(stored_wallet.len(), 1);

        let login = stored_wallet.get_encrypted_login_by_index(0).unwrap();
        assert_eq!(login.id, id1);

        // some actual ciphertext was saved
        assert!(!login.account.ciphertext().is_empty());
    }

    #[test]
    fn store_single_login_with_multi_then_update_pwd_and_load() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1,
            cosmos_hd_path,
            id1.clone(),
            &password,
        )
        .unwrap();

        update_encrypted_logins_at_file(&wallet_file, &password, &new_password).unwrap();

        let stored_wallet = load_existing_wallet_at_file(&wallet_file).unwrap();
        assert_eq!(stored_wallet.len(), 1);

        let login = stored_wallet.get_encrypted_login_by_index(0).unwrap();
        assert_eq!(login.id, id1);

        // some actual ciphertext was saved
        assert!(!login.account.ciphertext().is_empty());
    }

    #[test]
    fn store_twice_for_the_same_id_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        // Store the first login
        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        // and storing the same id again fails
        assert!(matches!(
            store_login_at_file(&wallet_file, account1, hd_path, id1, &password,),
            Err(BackendError::WalletLoginIdAlreadyExists),
        ));
    }

    #[test]
    fn store_twice_for_the_same_id_fails_with_multiple() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        // Store the first login
        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        // and storing the same id again fails
        assert!(matches!(
            store_login_with_multiple_accounts_at_file(
                &wallet_file,
                account1,
                hd_path,
                id1,
                &password,
            ),
            Err(BackendError::WalletLoginIdAlreadyExists),
        ));
    }

    #[test]
    fn load_with_wrong_password_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(&wallet_file, account1, hd_path, id1.clone(), &password).unwrap();

        // Trying to load it with wrong password now fails
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id1, &bad_password),
            Err(BackendError::StoreCipherError { .. }),
        ));
    }

    #[test]
    fn load_with_wrong_password_fails_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1,
            hd_path,
            id1.clone(),
            &password,
        )
        .unwrap();

        // Trying to load it with wrong password now fails
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id1, &bad_password),
            Err(BackendError::StoreCipherError { .. }),
        ));
    }

    #[test]
    fn load_with_wrong_id_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        store_login_at_file(&wallet_file, account1, hd_path, id1, &password).unwrap();

        // Trying to load with the wrong id
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id2, &password),
            Err(BackendError::WalletNoSuchLoginId),
        ));
    }

    #[test]
    fn load_with_wrong_id_fails_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(&wallet_file, account1, hd_path, id1, &password)
            .unwrap();

        // Trying to load with the wrong id
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id2, &password),
            Err(BackendError::WalletNoSuchLoginId),
        ));
    }

    #[test]
    fn store_and_load_a_single_login() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc.mnemonic());
        assert_eq!(&hd_path, acc.hd_path());
    }

    #[test]
    fn store_a_single_login_then_update_pwd_and_load() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        update_encrypted_logins_at_file(&wallet_file, &password, &new_password).unwrap();

        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &new_password).unwrap();
        let acc = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc.mnemonic());
        assert_eq!(&hd_path, acc.hd_path());
    }

    #[test]
    fn store_a_single_login_then_update_pwd_with_identical_pwd_is_noop_but_okay() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        update_encrypted_logins_at_file(&wallet_file, &password, &password).unwrap();

        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc.mnemonic());
        assert_eq!(&hd_path, acc.hd_path());
    }

    #[test]
    fn store_a_single_login_then_update_pwd_with_wrong_current_pwd_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let wrong_password = UserPassword::new("wrong_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(&wallet_file, account1, hd_path, id1, &password).unwrap();

        let err = update_encrypted_logins_at_file(&wallet_file, &wrong_password, &new_password)
            .unwrap_err();
        assert!(matches!(err, BackendError::StoreCipherError { .. }));
    }

    #[test]
    fn store_a_single_login_then_update_pwd_and_load_with_wrong_pwd_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_at_file(&wallet_file, account1, hd_path, id1.clone(), &password).unwrap();

        update_encrypted_logins_at_file(&wallet_file, &password, &new_password).unwrap();

        let err = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap_err();
        assert!(matches!(err, BackendError::StoreCipherError { .. }));
    }

    #[test]
    fn store_and_load_a_single_login_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let acc1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            acc1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let accounts = loaded_login.as_multiple_accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        let account = accounts
            .get_account(&DEFAULT_FIRST_ACCOUNT_NAME.into())
            .unwrap();
        assert_eq!(account.id().as_ref(), DEFAULT_FIRST_ACCOUNT_NAME);
        assert_eq!(account.mnemonic(), &acc1);
        assert_eq!(account.hd_path(), &hd_path);
    }

    #[test]
    fn store_a_single_login_with_multi_then_update_pwd_and_load() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let acc1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            acc1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        update_encrypted_logins_at_file(&wallet_file, &password, &new_password).unwrap();

        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &new_password).unwrap();
        let accounts = loaded_login.as_multiple_accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        let account = accounts
            .get_account(&DEFAULT_FIRST_ACCOUNT_NAME.into())
            .unwrap();
        assert_eq!(account.id().as_ref(), DEFAULT_FIRST_ACCOUNT_NAME);
        assert_eq!(account.mnemonic(), &acc1);
        assert_eq!(account.hd_path(), &hd_path);
    }

    #[test]
    fn store_a_single_login_with_multi_then_update_pwd_with_wrong_current_pwd_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let acc1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let wrong_password = UserPassword::new("wrong_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(&wallet_file, acc1, hd_path, id1, &password)
            .unwrap();

        let err = update_encrypted_logins_at_file(&wallet_file, &wrong_password, &new_password)
            .unwrap_err();
        assert!(matches!(err, BackendError::StoreCipherError { .. }));
    }

    #[test]
    fn store_a_single_login_with_multi_then_update_pwd_and_load_with_wrong_pwd_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let acc1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let id1 = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            acc1,
            hd_path,
            id1.clone(),
            &password,
        )
        .unwrap();

        update_encrypted_logins_at_file(&wallet_file, &password, &new_password).unwrap();

        let err = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap_err();
        assert!(matches!(err, BackendError::StoreCipherError { .. }));
    }

    #[test]
    fn store_a_second_login_with_a_different_password_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        store_login_at_file(
            &wallet_file,
            account1,
            cosmos_hd_path.clone(),
            id1,
            &password,
        )
        .unwrap();

        // Can't store a second login if you use different password
        assert!(matches!(
            store_login_at_file(&wallet_file, account2, cosmos_hd_path, id2, &bad_password),
            Err(BackendError::WalletDifferentPasswordDetected),
        ));
    }

    #[test]
    fn store_a_second_login_with_a_different_password_fails_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1,
            hd_path.clone(),
            id1,
            &password,
        )
        .unwrap();

        // Can't store a second login if you use different password
        assert!(matches!(
            store_login_with_multiple_accounts_at_file(
                &wallet_file,
                account2,
                hd_path,
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
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let different_hd_path: DerivationPath = "m".parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        // Store the first account
        store_login_at_file(&wallet_file, account1, hd_path, id1, &password).unwrap();

        let stored_wallet = load_existing_wallet_at_file(&wallet_file).unwrap();
        let encrypted_blob = &stored_wallet
            .get_encrypted_login_by_index(0)
            .unwrap()
            .account;

        // keep track of salt and iv for future assertion
        let original_iv = encrypted_blob.iv().to_vec();
        let original_salt = encrypted_blob.salt().to_vec();

        // Add an extra account
        store_login_at_file(&wallet_file, account2, different_hd_path, id2, &password).unwrap();

        let loaded_accounts = load_existing_wallet_at_file(&wallet_file).unwrap();
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
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let different_hd_path: DerivationPath = "m".parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        // Store the first account
        store_login_at_file(
            &wallet,
            account1.clone(),
            cosmos_hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        let login = load_existing_login_at_file(&wallet, &id1, &password).unwrap();
        let acc = login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc.mnemonic());
        assert_eq!(&cosmos_hd_path, acc.hd_path());

        // Add an extra account
        store_login_at_file(
            &wallet,
            account2.clone(),
            different_hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // first account should be unchanged
        let loaded_login = load_existing_login_at_file(&wallet, &id1, &password).unwrap();
        let acc1 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc1.mnemonic());
        assert_eq!(&cosmos_hd_path, acc1.hd_path());

        let loaded_login = load_existing_login_at_file(&wallet, &id2, &password).unwrap();
        let acc2 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account2, acc2.mnemonic());
        assert_eq!(&different_hd_path, acc2.hd_path());
    }

    #[test]
    fn store_two_mnemonic_accounts_using_two_logins_then_update_pwd_and_load() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let different_hd_path: DerivationPath = "m".parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        // Store the first login with an account
        store_login_at_file(
            &wallet,
            account1.clone(),
            cosmos_hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        let login = load_existing_login_at_file(&wallet, &id1, &password).unwrap();
        let acc = login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc.mnemonic());
        assert_eq!(&cosmos_hd_path, acc.hd_path());

        // Store a second login, also with an account
        store_login_at_file(
            &wallet,
            account2.clone(),
            different_hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        update_encrypted_logins_at_file(&wallet, &password, &new_password).unwrap();

        // first account should be unchanged
        let loaded_login = load_existing_login_at_file(&wallet, &id1, &new_password).unwrap();
        let acc1 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc1.mnemonic());
        assert_eq!(&cosmos_hd_path, acc1.hd_path());

        let loaded_login = load_existing_login_at_file(&wallet, &id2, &new_password).unwrap();
        let acc2 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account2, acc2.mnemonic());
        assert_eq!(&different_hd_path, acc2.hd_path());
    }

    #[test]
    fn store_one_mnemonic_account_and_one_multi_account() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let different_hd_path: DerivationPath = "m".parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        // Store the first account
        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc.mnemonic());
        assert_eq!(&hd_path, acc.hd_path());

        // Add an extra account
        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account2.clone(),
            different_hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // first account should be unchanged
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc1 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc1.mnemonic());
        assert_eq!(&hd_path, acc1.hd_path());

        let loaded_login = load_existing_login_at_file(&wallet_file, &id2, &password).unwrap();
        let acc2 = loaded_login.as_multiple_accounts().unwrap();
        assert_eq!(acc2.len(), 1);
        let account = acc2
            .get_account(&DEFAULT_FIRST_ACCOUNT_NAME.into())
            .unwrap();
        assert_eq!(account.id().as_ref(), DEFAULT_FIRST_ACCOUNT_NAME);
        assert_eq!(account.mnemonic(), &account2);
        assert_eq!(account.hd_path(), &different_hd_path);
    }

    #[test]
    fn remove_non_existent_id_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(&wallet_file, account1, hd_path, id1, &password)
            .unwrap();

        // Fails to delete non-existent id in the wallet
        assert!(matches!(
            remove_login_at_file(&wallet_file, &id2),
            Err(BackendError::WalletNoSuchLoginId),
        ));
    }

    #[test]
    fn store_and_remove_wallet_login_information() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let cosmos_hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let different_hd_path: DerivationPath = "m".parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        // Store two accounts with two different passwords
        store_login_at_file(
            &wallet_file,
            account1.clone(),
            cosmos_hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();
        store_login_at_file(
            &wallet_file,
            account2.clone(),
            different_hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // Load and compare
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc1 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc1.mnemonic());
        assert_eq!(&cosmos_hd_path, acc1.hd_path());

        let loaded_login = load_existing_login_at_file(&wallet_file, &id2, &password).unwrap();
        let acc2 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account2, acc2.mnemonic());
        assert_eq!(&different_hd_path, acc2.hd_path());

        // Delete the second account
        remove_login_at_file(&wallet_file, &id2).unwrap();

        // The first account should be unchanged
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc1 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(&account1, acc1.mnemonic());
        assert_eq!(&cosmos_hd_path, acc1.hd_path());

        // And we can't load the second one anymore
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id2, &password),
            Err(BackendError::WalletNoSuchLoginId),
        ));

        // Delete the first account
        assert!(wallet_file.exists());
        remove_login_at_file(&wallet_file, &id1).unwrap();

        // The file should now be removed
        assert!(!wallet_file.exists());

        // And trying to load when the file is gone fails
        assert!(matches!(
            load_existing_login_at_file(&wallet_file, &id1, &password),
            Err(BackendError::WalletFileNotFound),
        ));
    }

    #[test]
    fn append_account_converts_the_type() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = AccountId::new("second".to_string());

        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        // Check that it's there as the correct non-multiple type
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(acc.mnemonic(), &account1);
        assert_eq!(acc.hd_path(), &hd_path);

        append_account_to_login_at_file(
            &wallet_file,
            account2.clone(),
            hd_path.clone(),
            id1.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // Check that it is now multiple mnemonic type
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(id1.into(), MnemonicAccount::new(account1, hd_path.clone())),
            WalletAccount::new(id2, MnemonicAccount::new(account2, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    #[test]
    fn append_accounts_to_existing_login() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let account3 = Mnemonic::generate(24).unwrap();
        let account4 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());
        let id3 = AccountId::new("third".to_string());
        let id4 = AccountId::new("fourth".to_string());

        store_login_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        store_login_at_file(
            &wallet_file,
            account2.clone(),
            hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // Check that it's there as the correct non-multiple type
        let loaded_login = load_existing_login_at_file(&wallet_file, &id2, &password).unwrap();
        let acc2 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(acc2.mnemonic(), &account2);
        assert_eq!(acc2.hd_path(), &hd_path);

        // Add a third and fourth mnenonic grouped together with the second one
        append_account_to_login_at_file(
            &wallet_file,
            account3.clone(),
            hd_path.clone(),
            id2.clone(),
            id3.clone(),
            &password,
        )
        .unwrap();
        append_account_to_login_at_file(
            &wallet_file,
            account4.clone(),
            hd_path.clone(),
            id2.clone(),
            id4.clone(),
            &password,
        )
        .unwrap();

        // Check that we can load all four
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let acc1 = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(acc1.mnemonic(), &account1);
        assert_eq!(acc1.hd_path(), &hd_path);

        let loaded_login = load_existing_login_at_file(&wallet_file, &id2, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(id2.into(), MnemonicAccount::new(account2, hd_path.clone())),
            WalletAccount::new(id3, MnemonicAccount::new(account3, hd_path.clone())),
            WalletAccount::new(id4, MnemonicAccount::new(account4, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    #[test]
    fn append_accounts_to_existing_login_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let account3 = Mnemonic::generate(24).unwrap();
        let account4 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());
        let id3 = AccountId::new("third".to_string());
        let id4 = AccountId::new("fourth".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account2.clone(),
            hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // Add a third and fourth mnenonic grouped together with the second one
        append_account_to_login_at_file(
            &wallet_file,
            account3.clone(),
            hd_path.clone(),
            id2.clone(),
            id3.clone(),
            &password,
        )
        .unwrap();
        append_account_to_login_at_file(
            &wallet_file,
            account4.clone(),
            hd_path.clone(),
            id2.clone(),
            id4.clone(),
            &password,
        )
        .unwrap();

        // Check that we can load all four
        let loaded_login = load_existing_login_at_file(&wallet_file, &id1, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![WalletAccount::new(
            DEFAULT_FIRST_ACCOUNT_NAME.into(),
            MnemonicAccount::new(account1, hd_path.clone()),
        )]
        .into();
        assert_eq!(loaded_accounts, &expected);

        let loaded_login = load_existing_login_at_file(&wallet_file, &id2, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                DEFAULT_FIRST_ACCOUNT_NAME.into(),
                MnemonicAccount::new(account2, hd_path.clone()),
            ),
            WalletAccount::new(id3, MnemonicAccount::new(account3, hd_path.clone())),
            WalletAccount::new(id4, MnemonicAccount::new(account4, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    #[test]
    fn append_account_to_existing_login_with_multi_then_update_pwd_and_load() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let new_password = UserPassword::new("new_password".to_string());
        let login_id = LoginId::new("first".to_string());
        let appended_account = AccountId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            login_id.clone(),
            &password,
        )
        .unwrap();

        // Append a second mnenonic to the same login
        append_account_to_login_at_file(
            &wallet_file,
            account2.clone(),
            hd_path.clone(),
            login_id.clone(),
            appended_account.clone(),
            &password,
        )
        .unwrap();

        // Update the password
        update_encrypted_logins_at_file(&wallet_file, &password, &new_password).unwrap();

        // Expect that we can load these 2 accounts with the new password
        let loaded_login =
            load_existing_login_at_file(&wallet_file, &login_id, &new_password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                DEFAULT_FIRST_ACCOUNT_NAME.into(),
                MnemonicAccount::new(account1, hd_path.clone()),
            ),
            WalletAccount::new(appended_account, MnemonicAccount::new(account2, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);

        // Expect that trying to load these 2 accounts with the old password fails
        let err = load_existing_login_at_file(&wallet_file, &login_id, &password).unwrap_err();
        assert!(matches!(err, BackendError::StoreCipherError { .. }));
    }

    #[test]
    fn append_the_same_mnemonic_twice_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = AccountId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        assert!(matches!(
            append_account_to_login_at_file(&wallet_file, account1, hd_path, id1, id2, &password),
            Err(BackendError::WalletMnemonicAlreadyExistsInWalletLogin),
        ))
    }

    #[test]
    fn append_the_same_account_name_twice_fails() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let mnemonic1 = Mnemonic::generate(24).unwrap();
        let mnemonic2 = Mnemonic::generate(24).unwrap();
        let mnemonic3 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        // The top-level login id. NOTE: the first account id is always set to default.
        let login_id = LoginId::new("my_login_id".to_string());

        // Store the first account under login_id. The first account id is always set to default
        // name.
        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            mnemonic1,
            hd_path.clone(),
            login_id.clone(),
            &password,
        )
        .unwrap();

        // Append another account (account2) to the same login (login_id)
        let account2 = AccountId::new("account_2".to_string());

        append_account_to_login_at_file(
            &wallet_file,
            mnemonic2,
            hd_path.clone(),
            login_id.clone(),
            account2.clone(),
            &password,
        )
        .unwrap();

        // Appending the third account, with same account id will fail
        assert!(matches!(
            append_account_to_login_at_file(
                &wallet_file,
                mnemonic3,
                hd_path,
                login_id,
                account2,
                &password,
            ),
            Err(BackendError::WalletAccountIdAlreadyExistsInWalletLogin),
        ))
    }

    #[test]
    fn delete_the_same_account_twice_for_a_login_fails() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = AccountId::new("second".to_string());

        store_login_at_file(&wallet, account1, hd_path.clone(), id1.clone(), &password).unwrap();

        append_account_to_login_at_file(
            &wallet,
            account2,
            hd_path,
            id1.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        remove_account_from_login_at_file(&wallet, &id1, &id2, &password).unwrap();

        assert!(matches!(
            remove_account_from_login_at_file(&wallet, &id1, &id2, &password),
            Err(BackendError::WalletNoSuchAccountIdInWalletLogin),
        ));
    }

    #[test]
    fn delete_the_same_account_twice_for_a_login_fails_with_multi() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = AccountId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet_file,
            account1,
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        append_account_to_login_at_file(
            &wallet_file,
            account2,
            hd_path,
            id1.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        remove_account_from_login_at_file(&wallet_file, &id1, &id2, &password).unwrap();

        assert!(matches!(
            remove_account_from_login_at_file(&wallet_file, &id1, &id2, &password),
            Err(BackendError::WalletNoSuchAccountIdInWalletLogin),
        ));
    }

    #[test]
    fn delete_appended_account_doesnt_affect_others() {
        let store_dir = tempdir().unwrap();
        let wallet_file = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let account3 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());
        let id3 = AccountId::new("third".to_string());

        store_login_at_file(
            &wallet_file,
            account1,
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        store_login_at_file(
            &wallet_file,
            account2.clone(),
            hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        append_account_to_login_at_file(
            &wallet_file,
            account3.clone(),
            hd_path.clone(),
            id2.clone(),
            id3.clone(),
            &password,
        )
        .unwrap();

        remove_login_at_file(&wallet_file, &id1).unwrap();

        // The second login one is still there
        let loaded_login = load_existing_login_at_file(&wallet_file, &id2, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(id2.into(), MnemonicAccount::new(account2, hd_path.clone())),
            WalletAccount::new(id3, MnemonicAccount::new(account3, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    #[test]
    fn remove_all_accounts_for_a_login_removes_the_file_when_empty() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = AccountId::new("second".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet,
            account1,
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        append_account_to_login_at_file(
            &wallet,
            account2,
            hd_path,
            id1.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        remove_account_from_login_at_file(
            &wallet,
            &id1,
            &DEFAULT_FIRST_ACCOUNT_NAME.into(),
            &password,
        )
        .unwrap();
        remove_account_from_login_at_file(&wallet, &id1, &id2, &password).unwrap();

        // The file should now be removed
        assert!(!wallet.exists());

        // And trying to load when the file is gone fails
        assert!(matches!(
            load_existing_login_at_file(&wallet, &id1, &password),
            Err(BackendError::WalletFileNotFound),
        ));
    }

    #[test]
    fn remove_all_accounts_for_a_login_removes_that_login() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let account3 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = AccountId::new("second".to_string());
        let id3 = LoginId::new("third".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet,
            account1,
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        append_account_to_login_at_file(
            &wallet,
            account2,
            hd_path.clone(),
            id1.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        store_login_with_multiple_accounts_at_file(
            &wallet,
            account3.clone(),
            hd_path.clone(),
            id3.clone(),
            &password,
        )
        .unwrap();

        remove_account_from_login_at_file(
            &wallet,
            &id1,
            &DEFAULT_FIRST_ACCOUNT_NAME.into(),
            &password,
        )
        .unwrap();
        remove_account_from_login_at_file(&wallet, &id1, &id2, &password).unwrap();

        // And trying to load when the file is gone fails
        assert!(matches!(
            load_existing_login_at_file(&wallet, &id1, &password),
            Err(BackendError::WalletNoSuchLoginId),
        ));

        // The other login is still there
        let loaded_login = load_existing_login_at_file(&wallet, &id3, &password).unwrap();
        let acc3 = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![WalletAccount::new(
            DEFAULT_FIRST_ACCOUNT_NAME.into(),
            MnemonicAccount::new(account3, hd_path),
        )]
        .into();
        assert_eq!(acc3, &expected);
    }

    #[test]
    fn append_accounts_and_remove_appended_accounts() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let acc1 = Mnemonic::generate(24).unwrap();
        let acc2 = Mnemonic::generate(24).unwrap();
        let acc3 = Mnemonic::generate(24).unwrap();
        let acc4 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());
        let id3 = AccountId::new("third".to_string());
        let id4 = AccountId::new("fourth".to_string());

        store_login_at_file(
            &wallet,
            acc1.clone(),
            hd_path.clone(),
            id1.clone(),
            &password,
        )
        .unwrap();

        store_login_at_file(
            &wallet,
            acc2.clone(),
            hd_path.clone(),
            id2.clone(),
            &password,
        )
        .unwrap();

        // Add a third and fourth mnenonic grouped together with the second one
        append_account_to_login_at_file(
            &wallet,
            acc3,
            hd_path.clone(),
            id2.clone(),
            id3.clone(),
            &password,
        )
        .unwrap();
        append_account_to_login_at_file(
            &wallet,
            acc4.clone(),
            hd_path.clone(),
            id2.clone(),
            id4.clone(),
            &password,
        )
        .unwrap();

        // Delete the third mnemonic, from the second login entry
        remove_account_from_login_at_file(&wallet, &id2, &id3, &password).unwrap();

        // Check that we can still load the other accounts
        let loaded_login = load_existing_login_at_file(&wallet, &id2, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                id2.clone().into(),
                MnemonicAccount::new(acc2, hd_path.clone()),
            ),
            WalletAccount::new(id4.clone(), MnemonicAccount::new(acc4, hd_path.clone())),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);

        // Delete the second and fourth mnemonic from the second login entry removes the login entry
        remove_account_from_login_at_file(&wallet, &id2, &id2.clone().into(), &password).unwrap();
        remove_account_from_login_at_file(&wallet, &id2, &id4, &password).unwrap();
        assert!(matches!(
            load_existing_login_at_file(&wallet, &id2, &password),
            Err(BackendError::WalletNoSuchLoginId),
        ));

        // The first login is still available
        let loaded_login = load_existing_login_at_file(&wallet, &id1, &password).unwrap();
        let account = loaded_login.as_mnemonic_account().unwrap();
        assert_eq!(account.mnemonic(), &acc1);
        assert_eq!(account.hd_path(), &hd_path);
    }

    #[test]
    fn rename_first_account_in_login() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let login_id = LoginId::new("first".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet,
            account1.clone(),
            hd_path.clone(),
            login_id.clone(),
            &password,
        )
        .unwrap();

        let loaded_login = load_existing_login_at_file(&wallet, &login_id, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![WalletAccount::new(
            DEFAULT_FIRST_ACCOUNT_NAME.into(),
            MnemonicAccount::new(account1.clone(), hd_path.clone()),
        )]
        .into();
        assert_eq!(loaded_accounts, &expected);

        let renamed_account = AccountId::new("new_first".to_string());

        rename_account_in_login_at_file(
            &wallet,
            &login_id,
            &DEFAULT_FIRST_ACCOUNT_NAME.into(),
            &renamed_account,
            &password,
        )
        .unwrap();

        let loaded_login = load_existing_login_at_file(&wallet, &login_id, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![WalletAccount::new(
            renamed_account,
            MnemonicAccount::new(account1, hd_path),
        )]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    #[test]
    fn rename_one_account_in_login_with_two_accounts() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let login_id = LoginId::new("first".to_string());
        let account_id2 = AccountId::new("second".to_string());
        let renamed_account_id2 = AccountId::new("new_second".to_string());

        store_login_with_multiple_accounts_at_file(
            &wallet,
            account1.clone(),
            hd_path.clone(),
            login_id.clone(),
            &password,
        )
        .unwrap();

        append_account_to_login_at_file(
            &wallet,
            account2.clone(),
            hd_path.clone(),
            login_id.clone(),
            account_id2.clone(),
            &password,
        )
        .unwrap();

        // Load and confirm
        let loaded_login = load_existing_login_at_file(&wallet, &login_id, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                DEFAULT_FIRST_ACCOUNT_NAME.into(),
                MnemonicAccount::new(account1.clone(), hd_path.clone()),
            ),
            WalletAccount::new(
                account_id2.clone(),
                MnemonicAccount::new(account2.clone(), hd_path.clone()),
            ),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);

        // Rename the second account to a new name
        rename_account_in_login_at_file(
            &wallet,
            &login_id,
            &account_id2,
            &renamed_account_id2,
            &password,
        )
        .unwrap();

        // Load and confirm
        let loaded_login = load_existing_login_at_file(&wallet, &login_id, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                DEFAULT_FIRST_ACCOUNT_NAME.into(),
                MnemonicAccount::new(account1, hd_path.clone()),
            ),
            WalletAccount::new(renamed_account_id2, MnemonicAccount::new(account2, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    #[test]
    fn rename_account_into_existing_account_id_fails() {
        let store_dir = tempdir().unwrap();
        let wallet = store_dir.path().join(WALLET_INFO_FILENAME);
        let account1 = Mnemonic::generate(24).unwrap();
        let account2 = Mnemonic::generate(24).unwrap();
        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let login_id = LoginId::new("first".to_string());
        let account_id2 = AccountId::new("second".to_string());
        let renamed_account_id2 = DEFAULT_FIRST_ACCOUNT_NAME.into();

        store_login_with_multiple_accounts_at_file(
            &wallet,
            account1.clone(),
            hd_path.clone(),
            login_id.clone(),
            &password,
        )
        .unwrap();

        append_account_to_login_at_file(
            &wallet,
            account2.clone(),
            hd_path.clone(),
            login_id.clone(),
            account_id2.clone(),
            &password,
        )
        .unwrap();

        // Load and confirm
        let loaded_login = load_existing_login_at_file(&wallet, &login_id, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                DEFAULT_FIRST_ACCOUNT_NAME.into(),
                MnemonicAccount::new(account1.clone(), hd_path.clone()),
            ),
            WalletAccount::new(
                account_id2.clone(),
                MnemonicAccount::new(account2.clone(), hd_path.clone()),
            ),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);

        // Rename the second account to the name of the first one fails
        assert!(matches!(
            rename_account_in_login_at_file(
                &wallet,
                &login_id,
                &account_id2,
                &renamed_account_id2,
                &password,
            ),
            Err(BackendError::WalletAccountIdAlreadyExistsInWalletLogin)
        ));

        // Load and confirm nothing was changed
        let loaded_login = load_existing_login_at_file(&wallet, &login_id, &password).unwrap();
        let loaded_accounts = loaded_login.as_multiple_accounts().unwrap();
        let expected = vec![
            WalletAccount::new(
                DEFAULT_FIRST_ACCOUNT_NAME.into(),
                MnemonicAccount::new(account1, hd_path.clone()),
            ),
            WalletAccount::new(account_id2, MnemonicAccount::new(account2, hd_path)),
        ]
        .into();
        assert_eq!(loaded_accounts, &expected);
    }

    // Test to that decrypts a stored file from the repo, to make sure we are able to decrypt stored
    // wallets created with older versions.
    #[test]
    fn decrypt_stored_wallet() {
        const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet.json";
        let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);

        let wallet = load_existing_wallet_at_file(&wallet_file).unwrap();

        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let id1 = LoginId::new("first".to_string());
        let id2 = LoginId::new("second".to_string());

        assert!(!wallet.password_can_decrypt_all(&bad_password));
        assert!(wallet.password_can_decrypt_all(&password));

        let acc1 = wallet.decrypt_login(&id1, &password).unwrap();
        let acc2 = wallet.decrypt_login(&id2, &password).unwrap();

        assert!(matches!(acc1, StoredLogin::Mnemonic(_)));
        assert!(matches!(acc2, StoredLogin::Mnemonic(_)));

        let expected_acc1 = bip39::Mnemonic::from_str("country mean universe text phone begin deputy reject result good cram illness common cluster proud swamp digital patrol spread bar face december base kick").unwrap();
        let expected_acc2 =  bip39::Mnemonic::from_str("home mansion start quiz dress decide hint second dragon sunny juice always steak real minimum art rival skin draw total pulp foot goddess agent").unwrap();

        assert_eq!(
            acc1.as_mnemonic_account().unwrap().mnemonic(),
            &expected_acc1
        );
        assert_eq!(acc1.as_mnemonic_account().unwrap().hd_path(), &hd_path,);

        assert_eq!(
            acc2.as_mnemonic_account().unwrap().mnemonic(),
            &expected_acc2
        );
        assert_eq!(acc2.as_mnemonic_account().unwrap().hd_path(), &hd_path,);
    }

    #[test]
    fn decrypt_stored_wallet_1_0_4() {
        const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet-1.0.4.json";
        let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);

        let wallet = load_existing_wallet_at_file(&wallet_file).unwrap();

        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password11!".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let login_id = LoginId::new("default".to_string());

        assert!(!wallet.password_can_decrypt_all(&bad_password));
        assert!(wallet.password_can_decrypt_all(&password));

        let acc1 = wallet.decrypt_login(&login_id, &password).unwrap();

        assert!(matches!(acc1, StoredLogin::Mnemonic(_)));

        let expected_acc1 = bip39::Mnemonic::from_str("arrow capable abstract industry elevator nominee december piece hotel feed lounge web faint sword veteran bundle hour page actual laptop horror gold test warrior").unwrap();

        assert_eq!(
            acc1.as_mnemonic_account().unwrap().mnemonic(),
            &expected_acc1
        );
        assert_eq!(acc1.as_mnemonic_account().unwrap().hd_path(), &hd_path,);
    }

    #[test]
    fn decrypt_stored_wallet_1_0_5_with_multiple_accounts() {
        const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet-1.0.5.json";
        let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);

        let wallet = load_existing_wallet_at_file(&wallet_file).unwrap();

        let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
        let password = UserPassword::new("password11!".to_string());
        let bad_password = UserPassword::new("bad-password".to_string());
        let login_id = LoginId::new("default".to_string());

        assert!(!wallet.password_can_decrypt_all(&bad_password));
        assert!(wallet.password_can_decrypt_all(&password));

        let login = wallet.decrypt_login(&login_id, &password).unwrap();

        assert!(matches!(login, StoredLogin::Multiple(_)));

        let login = login.as_multiple_accounts().unwrap();
        assert_eq!(login.len(), 4);

        let expected_mn1 = bip39::Mnemonic::from_str("arrow capable abstract industry elevator nominee december piece hotel feed lounge web faint sword veteran bundle hour page actual laptop horror gold test warrior").unwrap();
        let expected_mn2 = bip39::Mnemonic::from_str("border hurt skull lunar goddess second danger game dismiss exhaust oven thumb dog drama onion false orchard spice tent next predict invite cherry green").unwrap();
        let expected_mn3 = bip39::Mnemonic::from_str("gentle crowd rule snap girl urge flat jump winner cluster night sand museum stock grunt quick tree acquire traffic major awake tag rack peasant").unwrap();
        let expected_mn4 = bip39::Mnemonic::from_str("debris blue skin annual inhale text border rigid spatial lesson coconut yard horn crystal control survey version vote hawk neck frame arrive oblige width").unwrap();

        let expected = vec![
            WalletAccount::new(
                "default".into(),
                MnemonicAccount::new(expected_mn1, hd_path.clone()),
            ),
            WalletAccount::new(
                "account2".into(),
                MnemonicAccount::new(expected_mn2, hd_path.clone()),
            ),
            WalletAccount::new(
                "foobar".into(),
                MnemonicAccount::new(expected_mn3, hd_path.clone()),
            ),
            WalletAccount::new("42".into(), MnemonicAccount::new(expected_mn4, hd_path)),
        ]
        .into();

        assert_eq!(login, &expected);
    }

    #[test]
    fn append_filename() {
        let wallet_file = PathBuf::from("/tmp/saved-wallet.json");
        let timestamp = OsString::from("42");
        #[cfg(target_family = "unix")]
        assert_eq!(
            append_timestamp_to_filename(wallet_file.clone(), timestamp.clone(), None)
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            "/tmp/saved-wallet-42.json".to_string(),
        );
        #[cfg(not(target_family = "unix"))]
        assert_eq!(
            append_timestamp_to_filename(wallet_file.clone(), timestamp.clone(), None)
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            r"/tmp\saved-wallet-42.json".to_string(),
        );

        #[cfg(target_family = "unix")]
        assert_eq!(
            append_timestamp_to_filename(wallet_file, timestamp, Some(3))
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            "/tmp/saved-wallet-42-3.json".to_string(),
        );
        #[cfg(not(target_family = "unix"))]
        assert_eq!(
            append_timestamp_to_filename(wallet_file, timestamp, Some(3))
                .unwrap()
                .into_os_string()
                .into_string()
                .unwrap(),
            r"/tmp\saved-wallet-42-3.json".to_string(),
        );
    }
}
