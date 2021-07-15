// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ValidatorClientError;
use cosmos_sdk::bip32::{DerivationPath, XPrv};
use cosmos_sdk::crypto::secp256k1::SigningKey;
use cosmos_sdk::crypto::PublicKey;
use cosmos_sdk::tx::SignDoc;
use cosmos_sdk::{tx, AccountId};

pub const DEFAULT_COSMOS_DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";
pub const DEFAULT_PREFIX: &str = "punk";

pub struct DirectSecp256k1HdWallet {
    secret: bip39::Mnemonic,
    seed: [u8; 64],
    accounts: Vec<AccountData>,
}

type Secp256k1Keypair = (SigningKey, PublicKey);

// this type feels weird. when implemented proper this should be re-thought
pub struct AccountData {
    address: AccountId,

    // note: since PublicKey is an enum, it already serves the purpose of the
    // export type Algo = "secp256k1" | "ed25519" | "sr25519" type from the cosmjs
    public_key: PublicKey,

    // I don't entirely understand why cosmjs split this off and put it in a separate `AccountDataWithPrivkey`
    // type.
    private_key: SigningKey,
}

// I've tried following cosmjs but some things were changed, for example we do not derive keys on every
// transaction we want to sign, we generate them once at construction.

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

    pub fn sign_direct(
        &self,
        signer_address: AccountId,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, ValidatorClientError> {
        let account = self
            .accounts
            .iter()
            .find(|account| account.address == signer_address)
            .ok_or(ValidatorClientError::SigningAccountNotFound(signer_address))?;

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
            prefix: DEFAULT_PREFIX.to_string(),
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
            seed,
        })
    }
}
