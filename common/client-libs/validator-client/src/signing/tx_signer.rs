// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::signer::{OfflineSigner, SigningError};
use crate::signing::SignerData;
use cosmrs::tx::{SignDoc, SignerInfo};
use cosmrs::{tx, AccountId, Any};

#[derive(Debug)]
/// A client that has only one responsibility - sign transactions
/// and not touch chain.
pub struct TxSigner<S> {
    signer: S,
}

impl<S> TxSigner<S> {
    pub fn new(signer: S) -> Self {
        TxSigner { signer }
    }

    pub fn signer(&self) -> &S {
        &self.signer
    }

    pub fn into_inner_signer(self) -> S {
        self.signer
    }

    pub fn sign_amino(
        &self,
        _signer_address: &AccountId,
        _messages: Vec<Any>,
        _fee: tx::Fee,
        _memo: impl Into<String> + Send + 'static,
        _signer_data: SignerData,
    ) -> Result<tx::Raw, S::Error>
    where
        S: OfflineSigner,
    {
        unimplemented!()
    }

    // TODO: change this sucker to use the trait better
    pub fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, S::Error>
    where
        S: OfflineSigner,
    {
        let account_from_signer = self.signer.find_account(signer_address)?;

        // TODO: experiment with this field
        let timeout_height = 0u32;

        let tx_body = tx::Body::new(messages, memo, timeout_height);
        let signer_info =
            SignerInfo::single_direct(Some(account_from_signer.public_key), signer_data.sequence);
        let auth_info = signer_info.auth_info(fee);

        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &signer_data.chain_id,
            signer_data.account_number,
        )
        .map_err(|source| SigningError::SignDocFailure { source }.into())?;

        self.signer
            .sign_direct_with_account(&account_from_signer, sign_doc)
    }
}
