// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::signer::{OfflineSigner, SigningError};
use crate::signing::{AccountData, Secp256k1Derivation};
use cosmrs::bip32::DerivationPath;
use cosmrs::tx;
use cosmrs::tx::SignDoc;
use nym_config::defaults;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Debug, Error)]
pub enum DirectSecp256k1HdWalletError {
    #[error(transparent)]
    SigningFailure(#[from] SigningError),

    #[error("failed to derive child key: {source}")]
    Bip32KeyDerivationFailure {
        #[from]
        source: bip32::Error,
    },

    #[error("There was an issue with bip39: {source}")]
    Bip39Error {
        #[from]
        source: bip39::Error,
    },

    #[error("failed to derive accounts: {source}")]
    AccountDerivationError { source: eyre::Report },
}

// TODO: maybe lock this one behind feature flag?
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DirectSecp256k1HdWallet {
    /// Base secret
    secret: bip39::Mnemonic,

    /// Derived accounts
    #[zeroize(skip)]
    // unfortunately `dyn EcdsaSigner` does not guarantee Zeroize
    accounts: Vec<AccountData>,
}

impl OfflineSigner for DirectSecp256k1HdWallet {
    type Error = DirectSecp256k1HdWalletError;

    fn get_accounts(&self) -> &[AccountData] {
        &self.accounts
    }

    fn sign_direct_with_account(
        &self,
        signer: &AccountData,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, Self::Error> {
        sign_doc
            .sign(&signer.private_key)
            .map_err(|source| SigningError::SigningFailure { source }.into())
    }
}

impl DirectSecp256k1HdWallet {
    pub fn builder(prefix: &str) -> DirectSecp256k1HdWalletBuilder {
        DirectSecp256k1HdWalletBuilder::new(prefix)
    }

    /// Restores a wallet from the given BIP39 mnemonic using default options.
    #[deprecated(
        note = "this function can potentially panic if accounts can't be derived correctly. please use .checked_from_mnemonic() instead"
    )]
    pub fn from_mnemonic(prefix: &str, mnemonic: bip39::Mnemonic) -> Self {
        // unfortunately due to backwards compatibility requirements,
        // we can't change signature of this method
        #[allow(deprecated)]
        DirectSecp256k1HdWalletBuilder::new(prefix).build(mnemonic)
    }

    /// Restores a wallet from the given BIP39 mnemonic using default options.
    pub fn checked_from_mnemonic(
        prefix: &str,
        mnemonic: bip39::Mnemonic,
    ) -> Result<Self, DirectSecp256k1HdWalletError> {
        DirectSecp256k1HdWalletBuilder::new(prefix).try_build(mnemonic)
    }

    pub fn generate(prefix: &str, word_count: usize) -> Result<Self, DirectSecp256k1HdWalletError> {
        let mneomonic = bip39::Mnemonic::generate(word_count)?;
        Self::checked_from_mnemonic(prefix, mneomonic)
    }

    pub fn secret(&self) -> &bip39::Mnemonic {
        &self.secret
    }

    #[deprecated(
        note = "use either .secret() for obtaining &bip39::Mnemonic or .mnemonic_string() for Zeroizing wrapper around the String"
    )]
    pub fn mnemonic(&self) -> String {
        self.secret.to_string()
    }

    pub fn mnemonic_string(&self) -> Zeroizing<String> {
        Zeroizing::new(self.secret.to_string())
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

    #[deprecated(
        note = "this function can potentially panic if accounts can't be derived correctly. please use .try_build() instead"
    )]
    pub fn build(self, mnemonic: bip39::Mnemonic) -> DirectSecp256k1HdWallet {
        // unfortunately due to backwards compatibility requirements,
        // we can't change signature of this method
        #[allow(clippy::expect_used)]
        self.try_build(mnemonic)
            .expect("account derivation failure")
    }

    pub fn try_build(
        self,
        mnemonic: bip39::Mnemonic,
    ) -> Result<DirectSecp256k1HdWallet, DirectSecp256k1HdWalletError> {
        let seed = Zeroizing::new(mnemonic.to_seed(&self.bip39_password));
        let prefix = self.prefix.clone();
        let accounts = self
            .hd_paths
            .iter()
            .map(|hd_path| {
                Secp256k1Derivation {
                    hd_path: hd_path.clone(),
                    prefix: prefix.clone(),
                }
                .try_derive_account(&seed)
            })
            .collect::<Result<_, _>>()?;

        Ok(DirectSecp256k1HdWallet {
            accounts,
            secret: mnemonic,
        })
    }
}

#[cfg(test)]
mod tests {
    use nym_network_defaults::NymNetworkDetails;

    use super::*;

    #[test]
    fn generating_account_addresses() -> anyhow::Result<()> {
        // test vectors produced from our js wallet
        let mnemonics = ["crush minute paddle tobacco message debate cabin peace bar jacket execute twenty winner view sure mask popular couch penalty fragile demise fresh pizza stove",
            "acquire rebel spot skin gun such erupt pull swear must define ill chief turtle today flower chunk truth battle claw rigid detail gym feel",
            "step income throw wheat mobile ship wave drink pool sudden upset jaguar bar globe rifle spice frost bless glimpse size regular carry aspect ball"];
        let prefix = NymNetworkDetails::new_mainnet()
            .chain_details
            .bech32_account_prefix;

        let addrs = [
            "n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf",
            "n1h5hgn94nsq4kh99rjj794hr5h5q6yfm2lr52es",
            "n17n9flp6jflljg6fp05dsy07wcprf2uuu8g40rf",
        ];
        for (idx, mnemonic) in mnemonics.iter().enumerate() {
            let wallet =
                DirectSecp256k1HdWallet::checked_from_mnemonic(&prefix, mnemonic.parse()?)?;
            assert_eq!(wallet.signer_addresses()[0], addrs[idx].parse().unwrap());
        }
        Ok(())
    }
}
