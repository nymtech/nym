// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::signer::{OfflineSigner, SigningError};
use crate::signing::SignerData;
use cosmrs::tx::{SignDoc, SignerInfo};
use cosmrs::{tx, AccountId, Any};

// extension trait for the OfflineSigner to allow to sign transactions
pub trait TxSigner: OfflineSigner {
    fn signer_public_key(&self, signer_address: &AccountId) -> Option<tx::SignerPublicKey> {
        let account = self.find_account(signer_address).ok()?;
        Some(account.public_key().into())
    }

    fn sign_amino(
        &self,
        _signer_address: &AccountId,
        _messages: Vec<Any>,
        _fee: tx::Fee,
        _memo: impl Into<String> + Send + 'static,
        _signer_data: SignerData,
    ) -> Result<tx::Raw, <Self as OfflineSigner>::Error> {
        unimplemented!()
    }

    fn sign_direct(
        &self,
        signer_address: &AccountId,
        messages: Vec<Any>,
        fee: tx::Fee,
        memo: impl Into<String> + Send + 'static,
        signer_data: SignerData,
    ) -> Result<tx::Raw, <Self as OfflineSigner>::Error> {
        let account_from_signer = self.find_account(signer_address)?;

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
        .map_err(|source| SigningError::SignDocFailure { source })?;

        self.sign_direct_with_account(&account_from_signer, sign_doc)
    }
}

impl<T> TxSigner for T where T: OfflineSigner {}
