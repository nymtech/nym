// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use config::defaults;
use cosmrs::bip32::{DerivationPath, XPrv};
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::crypto::PublicKey;
use cosmrs::tx::SignDoc;
use cosmrs::{tx, AccountId};

/// Derivation information required to derive a keypair and an address from a mnemonic.
#[derive(Debug, Clone)]
struct Secp256k1Derivation {
    hd_path: DerivationPath,
    prefix: String,
}

pub struct AccountData {
    pub(crate) address: AccountId,

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

type Secp256k1Keypair = (SigningKey, PublicKey);

#[derive(Debug, Clone)]
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
    pub fn from_mnemonic(mnemonic: bip39::Mnemonic) -> Result<Self, NymdError> {
        DirectSecp256k1HdWalletBuilder::new().build(mnemonic)
    }

    pub fn generate(word_count: usize) -> Result<Self, NymdError> {
        let mneomonic = bip39::Mnemonic::generate(word_count)?;
        Self::from_mnemonic(mneomonic)
    }

    fn derive_keypair(&self, hd_path: &DerivationPath) -> Result<Secp256k1Keypair, NymdError> {
        let extended_private_key = XPrv::derive_from_path(&self.seed, hd_path)?;

        let private_key: SigningKey = extended_private_key.into();
        let public_key = private_key.public_key();

        Ok((private_key, public_key))
    }

    pub fn try_derive_accounts(&self) -> Result<Vec<AccountData>, NymdError> {
        let mut accounts = Vec::with_capacity(self.accounts.len());
        for derivation_info in &self.accounts {
            let keypair = self.derive_keypair(&derivation_info.hd_path)?;

            // it seems this can only fail if the provided account prefix is invalid
            let address = keypair
                .1
                .account_id(&derivation_info.prefix)
                .map_err(|_| NymdError::AccountDerivationError)?;

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
    ) -> Result<tx::Raw, NymdError> {
        // ideally I'd prefer to have the entire error put into the NymdError::SigningFailure
        // but I'm super hesitant to trying to downcast the eyre::Report to cosmrs::error::Error
        sign_doc
            .sign(&signer.private_key)
            .map_err(|_| NymdError::SigningFailure)
    }

    pub fn sign_direct(
        &self,
        signer_address: &AccountId,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, NymdError> {
        // I hate deriving accounts at every sign here so much : (
        let accounts = self.try_derive_accounts()?;
        let account = accounts
            .iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| NymdError::SigningAccountNotFound(signer_address.clone()))?;

        self.sign_direct_with_account(account, sign_doc)
    }
}

#[must_use]
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

    pub fn build(self, mnemonic: bip39::Mnemonic) -> Result<DirectSecp256k1HdWallet, NymdError> {
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
    use network_defaults::BECH32_PREFIX;

    #[test]
    fn generating_account_addresses() {
        let (addr1, addr2, addr3) = match BECH32_PREFIX {
            "punk" => (
                "punk1jw6mp7d5xqc7w6xm79lha27glmd0vdt32a3fj2",
                "punk1h5hgn94nsq4kh99rjj794hr5h5q6yfm22mcqqn",
                "punk17n9flp6jflljg6fp05dsy07wcprf2uuujse962",
            ),
            "nymt" => (
                "nymt1jw6mp7d5xqc7w6xm79lha27glmd0vdt339me94",
                "nymt1h5hgn94nsq4kh99rjj794hr5h5q6yfm23rjshv",
                "nymt17n9flp6jflljg6fp05dsy07wcprf2uuufgn4d4",
            ),
            _ => panic!("Test needs to be updated with new bech32 prefix"),
        };
        // test vectors produced from our js wallet
        let mnemonic_address = vec![
            ("crush minute paddle tobacco message debate cabin peace bar jacket execute twenty winner view sure mask popular couch penalty fragile demise fresh pizza stove", addr1),
            ("acquire rebel spot skin gun such erupt pull swear must define ill chief turtle today flower chunk truth battle claw rigid detail gym feel", addr2),
            ("step income throw wheat mobile ship wave drink pool sudden upset jaguar bar globe rifle spice frost bless glimpse size regular carry aspect ball", addr3)
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
