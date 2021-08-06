// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use config::defaults;
use cosmos_sdk::bip32::{DerivationPath, XPrv};
use cosmos_sdk::crypto::secp256k1::SigningKey;
use cosmos_sdk::crypto::PublicKey;
use cosmos_sdk::tx::SignDoc;
use cosmos_sdk::{tx, AccountId};

/// Derivation information required to derive a keypair and an address from a mnemonic.
struct Secp256k1Derivation {
    hd_path: DerivationPath,
    prefix: String,
}

pub struct AccountData {
    pub(crate) address: AccountId,

    pub(crate) public_key: PublicKey,

    pub(crate) private_key: SigningKey,
}

type Secp256k1Keypair = (SigningKey, PublicKey);

pub struct DirectSecp256k1HdWallet {
    /// Base secret
    secret: bip39::Mnemonic,

    /// BIP39 seed
    seed: [u8; 64],

    // An unfortunate result of immature rust async story is that async traits (only available in the separate package)
    // can't yet figure out everything and if we stored our derived account data on the struct,
    // that would include the secret key which is a dyn EcdsaSigner and hence not Sync making the wallet
    // not Sync and if used on the signing client in an async trait, it wouldn't be Send
    /// Derivation instructions
    accounts: Vec<Secp256k1Derivation>,
}

impl DirectSecp256k1HdWallet {
    pub fn builder() -> DirectSecp256k1HdWalletBuilder {
        DirectSecp256k1HdWalletBuilder::default()
    }

    /// Restores a wallet from the given BIP39 mnemonic using default options.
    pub fn from_mnemonic(mnemonic: bip39::Mnemonic) -> Result<Self, ValidatorClientError> {
        DirectSecp256k1HdWalletBuilder::new().build(mnemonic)
    }

    pub fn generate(word_count: usize) -> Result<Self, ValidatorClientError> {
        let mneomonic = bip39::Mnemonic::generate(word_count)?;
        Self::from_mnemonic(mneomonic)
    }

    fn derive_keypair(
        &self,
        hd_path: &DerivationPath,
    ) -> Result<Secp256k1Keypair, ValidatorClientError> {
        let extended_private_key = XPrv::derive_from_path(&self.seed, hd_path)?;

        let private_key: SigningKey = extended_private_key.into();
        let public_key = private_key.public_key();

        Ok((private_key, public_key))
    }

    pub fn try_derive_accounts(&self) -> Result<Vec<AccountData>, ValidatorClientError> {
        let mut accounts = Vec::with_capacity(self.accounts.len());
        for derivation_info in &self.accounts {
            let keypair = self.derive_keypair(&derivation_info.hd_path)?;

            // it seems this can only fail if the provided account prefix is invalid
            let address = keypair
                .1
                .account_id(&derivation_info.prefix)
                .map_err(|_| ValidatorClientError::AccountDerivationError)?;

            accounts.push(AccountData {
                address,
                public_key: keypair.1,
                private_key: keypair.0,
            })
        }

        Ok(accounts)
    }

    pub fn mnemonic(&self) -> String {
        self.secret.to_string()
    }

    pub fn sign_direct_with_account(
        &self,
        signer: &AccountData,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, ValidatorClientError> {
        // ideally I'd prefer to have the entire error put into the ValidatorClientError::SigningFailure
        // but I'm super hesitant to trying to downcast the eyre::Report to cosmos_sdk::error::Error
        sign_doc
            .sign(&signer.private_key)
            .map_err(|_| ValidatorClientError::SigningFailure)
    }

    pub fn sign_direct(
        &self,
        signer_address: &AccountId,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, ValidatorClientError> {
        // I hate deriving accounts at every sign here so much : (
        let accounts = self.try_derive_accounts()?;
        let account = accounts
            .iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| ValidatorClientError::SigningAccountNotFound(signer_address.clone()))?;

        self.sign_direct_with_account(account, sign_doc)
    }
}

pub struct DirectSecp256k1HdWalletBuilder {
    /// The password to use when deriving a BIP39 seed from a mnemonic.
    bip39_password: String,

    /// The BIP-32/SLIP-10 derivation paths
    hd_paths: Vec<DerivationPath>,

    /// The bech32 address prefix (human readable part)
    prefix: String,
}

impl Default for DirectSecp256k1HdWalletBuilder {
    fn default() -> Self {
        DirectSecp256k1HdWalletBuilder {
            bip39_password: String::new(),
            hd_paths: vec![defaults::COSMOS_DERIVATION_PATH.parse().unwrap()],
            prefix: defaults::BECH32_PREFIX.to_string(),
        }
    }
}

impl DirectSecp256k1HdWalletBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_bip39_password<S: Into<String>>(mut self, password: S) -> Self {
        self.bip39_password = password.into();
        self
    }

    pub fn with_hd_path(mut self, path: DerivationPath) -> Self {
        self.hd_paths.push(path);
        self
    }

    pub fn with_hd_paths(mut self, hd_paths: Vec<DerivationPath>) -> Self {
        self.hd_paths = hd_paths;
        self
    }

    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn build(
        self,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSecp256k1HdWallet, ValidatorClientError> {
        let seed = mnemonic.to_seed(&self.bip39_password);
        let prefix = self.prefix;
        let accounts = self
            .hd_paths
            .into_iter()
            .map(|hd_path| Secp256k1Derivation {
                hd_path,
                prefix: prefix.clone(),
            })
            .collect();

        Ok(DirectSecp256k1HdWallet {
            accounts,
            seed,
            secret: mnemonic,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generating_account_addresses() {
        // test vectors produced from our js wallet
        let mnemonic_address = vec![
            ("crush minute paddle tobacco message debate cabin peace bar jacket execute twenty winner view sure mask popular couch penalty fragile demise fresh pizza stove", "punk1jw6mp7d5xqc7w6xm79lha27glmd0vdt32a3fj2"),
            ("acquire rebel spot skin gun such erupt pull swear must define ill chief turtle today flower chunk truth battle claw rigid detail gym feel", "punk1h5hgn94nsq4kh99rjj794hr5h5q6yfm22mcqqn"),
            ("step income throw wheat mobile ship wave drink pool sudden upset jaguar bar globe rifle spice frost bless glimpse size regular carry aspect ball", "punk17n9flp6jflljg6fp05dsy07wcprf2uuujse962")
        ];

        for (mnemonic, address) in mnemonic_address.into_iter() {
            let wallet = DirectSecp256k1HdWallet::from_mnemonic(mnemonic.parse().unwrap()).unwrap();
            assert_eq!(
                wallet.try_derive_accounts().unwrap()[0].address,
                address.parse().unwrap()
            )
        }
    }
}
