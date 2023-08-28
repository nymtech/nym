// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::AccountData;
pub use cosmrs::crypto::secp256k1::Signature;
use cosmrs::tx::SignDoc;
use cosmrs::{tx, AccountId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("the requested signing type: {typ:?} is not supported by this signer.")]
    UnsupportedSigningType { typ: SignerType },

    #[error("account {account} was not found within this signer")]
    AccountNotFound { account: AccountId },

    #[error("failed to sign the requested message: {source}")]
    SigningFailure { source: eyre::Report },

    #[error("failed to construct the sign doc: {source}")]
    SignDocFailure { source: eyre::Report },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SignerType {
    Amino,
    Direct,
}

// TODO: check if this trait needs to be async
// (I guess it depends on future ledger requirements)
pub trait OfflineSigner {
    type Error: From<SigningError>;

    // I really dislike existence of this function because it makes you re-derive your key **twice** for each contract transaction
    fn signer_addresses(&self) -> Result<Vec<AccountId>, Self::Error> {
        let derived_addresses = self
            .get_accounts()?
            .into_iter()
            .map(|account| account.address)
            .collect();
        Ok(derived_addresses)
    }

    fn get_accounts(&self) -> Result<Vec<AccountData>, Self::Error>;

    fn find_account(&self, signer_address: &AccountId) -> Result<AccountData, Self::Error> {
        // TODO: we could really use some zeroize action here
        let accounts = self.get_accounts()?;
        accounts
            .into_iter()
            .find(|account| &account.address == signer_address)
            .ok_or_else(|| {
                SigningError::AccountNotFound {
                    account: signer_address.clone(),
                }
                .into()
            })
    }

    fn sign_raw_with_account<M: AsRef<[u8]>>(
        &self,
        signer: &AccountData,
        message: M,
    ) -> Result<Signature, Self::Error> {
        signer
            .private_key
            .sign(message.as_ref())
            .map_err(|source| SigningError::SigningFailure { source }.into())
    }

    fn sign_raw<M: AsRef<[u8]>>(
        &self,
        signer_address: &AccountId,
        message: M,
    ) -> Result<Signature, Self::Error> {
        let signer = self.find_account(signer_address)?;
        self.sign_raw_with_account(&signer, message)
    }

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        sign_doc: SignDoc,
    ) -> Result<tx::Raw, Self::Error> {
        let signer = self.find_account(signer_address)?;
        self.sign_direct_with_account(&signer, sign_doc)
    }

    // unless explicitly defined, each signing method is unsupported
    fn sign_direct_with_account(
        &self,
        _signer: &AccountData,
        _sign_doc: SignDoc,
    ) -> Result<tx::Raw, Self::Error> {
        Err(SigningError::UnsupportedSigningType {
            typ: SignerType::Direct,
        }
        .into())
    }

    // fn sign_amino(&self, signer_address: &AccountId, sign_doc: AminoSignDoc) -> Result<tx::Raw, Self::Error>;

    // fn sign_amino_with_account(&self, signer: &AccountData, sign_doc: AminoSignDoc) -> Result<tx::Raw, Self::Error>;
}

#[derive(Debug, Default, Copy, Clone)]
pub struct NoSigner;

// #[derive(Debug, Copy, Clone, Error)]
// #[error("no signer is available")]
// struct SignerUnavailable;
//
// // trait bound requirements
// impl From<SigningError> for SignerUnavailable {
//     fn from(_: SigningError) -> Self {
//         SignerUnavailable
//     }
// }
//
// impl OfflineSigner for NoSigner {
//     type Error = SignerUnavailable;
//
//     fn get_accounts(&self) -> Result<Vec<AccountData>, Self::Error> {
//         return Err(SignerUnavailable);
//     }
// }
