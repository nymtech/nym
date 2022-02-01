// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::wallet_storage::account_data::StoredAccount;
use crate::wallet_storage::encryption::encrypt_struct;
use crate::wallet_storage::password::UserPassword;
use cosmrs::bip32::DerivationPath;

pub(crate) mod account_data;
pub(crate) mod encryption;
mod password;

pub(crate) struct Placeholder;

// pub(crate) fn store_wallet_login_information(
//   mnemonic: bip39::Mnemonic,
//   hd_path: DerivationPath,
//   password: UserPassword,
// ) -> Result<Placeholder, BackendError> {
//   let stored_account = StoredAccount::new_mnemonic_backed_account(mnemonic, hd_path);
//   let encrypted = encrypt_struct(&stored_account, &password)?;
//
//   // store encrypted on the disk
//   todo!("here be writing on the disk")
//
//   // as the function exits, password will be dropped and mnemonic will be overwritten with a fresh one
// }
