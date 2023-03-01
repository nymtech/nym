// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use cosmrs::bip32::{DerivationPath, XPrv};
use cosmrs::crypto::secp256k1::{Signature, SigningKey};
use cosmrs::crypto::PublicKey;
use cosmrs::tx::SignDoc;
use cosmrs::{tx, AccountId};
use nym_config::defaults;
use zeroize::Zeroize;

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

impl Zeroize for DirectSecp256k1HdWallet {
    fn zeroize(&mut self) {
        // in ideal world, Mnemonic would have had zeroize defined on it (there's an almost year old PR that introduces it)
        // and the memory would have been filled with zeroes.
        //
        // we really don't want to keep our real mnemonic in memory, so let's do the semi-nasty thing
        // of overwriting it with a fresh mnemonic that was never used before
        //
        // note: this function can only fail on an invalid word count, which clearly is not the case here
        self.secret = bip39::Mnemonic::generate(self.secret.word_count()).unwrap();
        self.seed.zeroize();
        // there's nothing secret about derivation paths
    }
}

impl Drop for DirectSecp256k1HdWallet {
    fn drop(&mut self) {
        self.zeroize()
    }
}

impl DirectSecp256k1HdWallet {
    pub fn builder(prefix: &str) -> DirectSecp256k1HdWalletBuilder {
        DirectSecp256k1HdWalletBuilder::new(prefix)
    }

    /// Restores a wallet from the given BIP39 mnemonic using default options.
    pub fn from_mnemonic(prefix: &str, mnemonic: bip39::Mnemonic) -> Self {
        DirectSecp256k1HdWalletBuilder::new(prefix).build(mnemonic)
    }

    pub fn generate(prefix: &str, word_count: usize) -> Result<Self, NyxdError> {
        let mneomonic = bip39::Mnemonic::generate(word_count)?;
        Ok(Self::from_mnemonic(prefix, mneomonic))
    }

    fn derive_keypair(&self, hd_path: &DerivationPath) -> Result<Secp256k1Keypair, NyxdError> {
        let extended_private_key = XPrv::derive_from_path(self.seed, hd_path)?;

        let private_key: SigningKey = extended_private_key.into();
        let public_key = private_key.public_key();

        Ok((private_key, public_key))
    }

    pub fn try_derive_accounts(&self) -> Result<Vec<AccountData>, NyxdError> {
        let mut accounts = Vec::with_capacity(self.accounts.len());
        for derivation_info in &self.accounts {
            let keypair = self.derive_keypair(&derivation_info.hd_path)?;

            // it seems this can only fail if the provided account prefix is invalid
            let address = keypair
                .1
                .account_id(&derivation_info.prefix)
                .map_err(|_| NyxdError::AccountDerivationError)?;

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

    pub fn sign_raw_with_account(
        &self,
        signer: &AccountData,
        message: &[u8],
    ) -> Result<Signature, NyxdError> {
        signer
            .private_key
            .sign(message)
            .map_err(|_| NyxdError::SigningFailure)
    }

    pub fn sign_direct_with_account(
        &self,
        signer: &AccountData,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, NyxdError> {
        // ideally I'd prefer to have the entire error put into the NyxdError::SigningFailure
        // but I'm super hesitant to trying to downcast the eyre::Report to cosmrs::error::Error
        sign_doc
            .sign(&signer.private_key)
            .map_err(|_| NyxdError::SigningFailure)
    }

    pub fn sign_direct(
        &self,
        signer_address: &AccountId,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, NyxdError> {
        // I hate deriving accounts at every sign here so much : (
        let accounts = self.try_derive_accounts()?;
        let account = accounts
            .iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| NyxdError::SigningAccountNotFound(signer_address.clone()))?;

        self.sign_direct_with_account(account, sign_doc)
    }
}

#[must_use]
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct DirectSecp256k1HdWalletBuilder {
    /// The password to use when deriving a BIP39 seed from a mnemonic.
    bip39_password: String,

    /// The BIP-32/SLIP-10 derivation paths
    #[zeroize(skip)]
    hd_paths: Vec<DerivationPath>,

    /// The bech32 address prefix (human readable part)
    prefix: String,
}

impl DirectSecp256k1HdWalletBuilder {
    pub fn new(prefix: &str) -> Self {
        DirectSecp256k1HdWalletBuilder {
            bip39_password: String::new(),
            hd_paths: vec![defaults::COSMOS_DERIVATION_PATH.parse().unwrap()],
            prefix: prefix.into(),
        }
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

    pub fn build(self, mnemonic: bip39::Mnemonic) -> DirectSecp256k1HdWallet {
        let seed = mnemonic.to_seed(&self.bip39_password);
        let prefix = self.prefix.clone();
        let accounts = self
            .hd_paths
            .iter()
            .map(|hd_path| Secp256k1Derivation {
                hd_path: hd_path.clone(),
                prefix: prefix.clone(),
            })
            .collect();

        DirectSecp256k1HdWallet {
            accounts,
            seed,
            secret: mnemonic,
        }
    }
}

#[cfg(test)]
mod tests {
    use nym_network_defaults::NymNetworkDetails;

    use super::*;

    #[test]
    fn generating_account_addresses() {
        // test vectors produced from our js wallet
        let mnemonics = vec![
            "crush minute paddle tobacco message debate cabin peace bar jacket execute twenty winner view sure mask popular couch penalty fragile demise fresh pizza stove",
            "acquire rebel spot skin gun such erupt pull swear must define ill chief turtle today flower chunk truth battle claw rigid detail gym feel",
            "step income throw wheat mobile ship wave drink pool sudden upset jaguar bar globe rifle spice frost bless glimpse size regular carry aspect ball"
        ];
        let prefix = NymNetworkDetails::new_mainnet()
            .chain_details
            .bech32_account_prefix;

        let addrs = vec![
            "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
            "n1h5hgn94nsq4kh99rjj794hr5h5q6yfm2lr52es",
            "n17n9flp6jflljg6fp05dsy07wcprf2uuu8g40rf",
        ];
        for (idx, mnemonic) in mnemonics.iter().enumerate() {
            let wallet = DirectSecp256k1HdWallet::from_mnemonic(&prefix, mnemonic.parse().unwrap());
            assert_eq!(
                wallet.try_derive_accounts().unwrap()[0].address,
                addrs[idx].parse().unwrap()
            )
        }
    }
}
