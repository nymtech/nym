// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use cosmos_sdk::bip32::{DerivationPath, XPrv};
use cosmos_sdk::crypto::secp256k1::SigningKey;
use cosmos_sdk::crypto::PublicKey;
use cosmos_sdk::tx::SignDoc;
use cosmos_sdk::{tx, AccountId};

pub const DEFAULT_COSMOS_DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";
pub const DEFAULT_BECH32_ADDRESS_PREFIX: &str = "punk";

pub struct DirectSecp256k1HdWallet {
    secret: bip39::Mnemonic,
    accounts: Vec<AccountData>,
}

type Secp256k1Keypair = (SigningKey, PublicKey);

pub struct AccountData {
    pub(crate) address: AccountId,

    // note: since PublicKey is an enum, it already serves the purpose of the
    // export type Algo = "secp256k1" | "ed25519" | "sr25519" type from the cosmjs
    pub(crate) public_key: PublicKey,

    // I don't entirely understand why cosmjs split this off and put it in a separate `AccountDataWithPrivkey`
    // type.
    // Note from future-self:
    // While this is not the reason they've done it, it might be potentially useful to introduce this in Rust,
    // as SigningKey is !Send here.
    // if we split it (and derived private key on every. single. sign request), we might possibly
    // be able to put it in a trait.
    private_key: SigningKey,
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

    pub fn mnemonic(&self) -> String {
        self.secret.to_string()
    }

    pub fn get_accounts(&self) -> &[AccountData] {
        &self.accounts
    }

    pub fn sign_direct(
        &self,
        signer_address: &AccountId,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, ValidatorClientError> {
        let account = self
            .accounts
            .iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| ValidatorClientError::SigningAccountNotFound(signer_address.clone()))?;

        // ideally I'd prefer to have the entire error put into the ValidatorClientError::SigningFailure
        // but I'm super hesitant to trying to downcast the eyre::Report to cosmos_sdk::error::Error
        sign_doc
            .sign(&account.private_key)
            .map_err(|_| ValidatorClientError::SigningFailure)
    }
}

pub struct DirectSecp256k1HdWalletBuilder {
    /// The password to use when deriving a BIP39 seed from a mnemonic.
    bip39_password: String,

    /// The BIP-32/SLIP-10 derivation paths. Defaults to the Cosmos Hub/ATOM path `m/44'/118'/0'/0/0`
    hd_paths: Vec<DerivationPath>,

    /// The bech32 address prefix (human readable part). Defaults to "punk".
    prefix: String,
}

impl Default for DirectSecp256k1HdWalletBuilder {
    fn default() -> Self {
        DirectSecp256k1HdWalletBuilder {
            bip39_password: String::new(),
            hd_paths: vec![DEFAULT_COSMOS_DERIVATION_PATH.parse().unwrap()],
            prefix: DEFAULT_BECH32_ADDRESS_PREFIX.to_string(),
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

    fn derive_keypair(
        seed: &[u8],
        hd_path: &DerivationPath,
    ) -> Result<Secp256k1Keypair, ValidatorClientError> {
        let extended_private_key = XPrv::derive_from_path(seed, hd_path)?;

        let private_key: SigningKey = extended_private_key.into();
        let public_key = private_key.public_key();

        Ok((private_key, public_key))
    }

    fn derive_accounts(&self, seed: &[u8]) -> Result<Vec<AccountData>, ValidatorClientError> {
        let mut accounts = Vec::with_capacity(self.hd_paths.len());

        for hd_path in self.hd_paths.iter() {
            let keypair = Self::derive_keypair(seed, hd_path)?;

            // it seems this can only fail if the provided account prefix is invalid
            let address = keypair
                .1
                .account_id(&self.prefix)
                .map_err(|_| ValidatorClientError::AccountDerivationError)?;

            accounts.push(AccountData {
                address,
                public_key: keypair.1,
                private_key: keypair.0,
            })
        }

        Ok(accounts)
    }

    pub fn build(
        self,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSecp256k1HdWallet, ValidatorClientError> {
        let seed = mnemonic.to_seed(&self.bip39_password);
        let accounts = self.derive_accounts(&seed)?;

        Ok(DirectSecp256k1HdWallet {
            accounts,
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
            assert_eq!(wallet.accounts[0].address, address.parse().unwrap())
        }
    }
}
