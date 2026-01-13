// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::DirectSecp256k1HdWallet;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::signing::signer::OfflineSigner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct Account {
    /// n1 address, e.g. 'n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy'
    pub(crate) address: AccountId,

    /// mnemonic associated with the account
    pub(crate) mnemonic: bip39::Mnemonic,
}

impl Account {
    // SAFETY: we're using valid constants
    #[allow(clippy::unwrap_used)]
    pub(crate) fn new() -> Account {
        let mnemonic = bip39::Mnemonic::generate(24).unwrap();
        let wallet = DirectSecp256k1HdWallet::checked_from_mnemonic("n", mnemonic.clone()).unwrap();
        let acc = wallet.get_accounts().first().unwrap();
        Account {
            address: acc.address.clone(),
            mnemonic,
        }
    }

    pub(crate) fn address(&self) -> AccountId {
        self.address.clone()
    }
}
