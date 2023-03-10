// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::bip32::DerivationPath;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::crypto::PublicKey;
use cosmrs::AccountId;

pub mod signer;
pub mod wallet;

/// Derivation information required to derive a keypair and an address from a mnemonic.
#[derive(Debug, Clone)]
struct Secp256k1Derivation {
    hd_path: DerivationPath,
    prefix: String,
}

// TODO: is this struct going to be derivable with other signer types?
pub struct AccountData {
    pub address: AccountId,

    pub(crate) public_key: PublicKey,

    pub(crate) private_key: SigningKey,
}

impl AccountData {
    pub fn address(&self) -> &AccountId {
        &self.address
    }

    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    pub fn private_key(&self) -> &SigningKey {
        &self.private_key
    }
}
