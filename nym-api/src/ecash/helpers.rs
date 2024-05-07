// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ecash::error::CoconutError;
use nym_api_requests::coconut::models::FreePassRequest;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_compact_ecash::scheme::coin_indices_signatures::{sign_coin_indices, CoinIndexSignature};
use nym_compact_ecash::scheme::expiration_date_signatures::{
    sign_expiration_date, ExpirationDateSignature,
};
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::BlindedSignature;
use nym_compact_ecash::{PublicKeyUser, VerificationKeyAuth, WithdrawalRequest};
use nym_validator_client::nyxd::error::NyxdError::AbciError;
use tokio::sync::RwLock;

// If the result is already established, the vote might be redundant and
// thus the transaction might fail
pub(crate) fn accepted_vote_err(ret: Result<(), CoconutError>) -> Result<(), CoconutError> {
    if let Err(CoconutError::NyxdError(AbciError { ref log, .. })) = ret {
        let accepted_err =
            nym_multisig_contract_common::error::ContractError::NotOpen {}.to_string();
        // If redundant voting is not the case, error out on all other error variants
        if !log.contains(&accepted_err) {
            ret?;
        }
    }
    Ok(())
}

pub(crate) trait CredentialRequest {
    fn withdrawal_request(&self) -> &WithdrawalRequest;
    fn expiration_date_timestamp(&self) -> u64;
    fn ecash_pubkey(&self) -> PublicKeyUser;
}

impl CredentialRequest for BlindSignRequestBody {
    fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.inner_sign_request
    }

    fn expiration_date_timestamp(&self) -> u64 {
        self.expiration_date.unix_timestamp() as u64
    }

    fn ecash_pubkey(&self) -> PublicKeyUser {
        self.ecash_pubkey.clone()
    }
}

impl CredentialRequest for FreePassRequest {
    fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.inner_sign_request
    }

    fn expiration_date_timestamp(&self) -> u64 {
        self.expiration_date.unix_timestamp() as u64
    }

    fn ecash_pubkey(&self) -> PublicKeyUser {
        self.ecash_pubkey.clone()
    }
}

pub(crate) fn blind_sign<C: CredentialRequest>(
    request: &C,
    signing_key: &SecretKeyAuth,
) -> Result<BlindedSignature, CoconutError> {
    Ok(nym_compact_ecash::scheme::withdrawal::issue(
        signing_key,
        request.ecash_pubkey().clone(),
        request.withdrawal_request(),
        request.expiration_date_timestamp(),
    )?)
}

pub(crate) struct CoinIndexSignatureCache {
    pub(crate) epoch_id: AtomicU64,
    pub(crate) signatures: RwLock<Option<Vec<CoinIndexSignature>>>,
}

impl CoinIndexSignatureCache {
    pub(crate) fn new() -> Self {
        CoinIndexSignatureCache {
            epoch_id: AtomicU64::new(u64::MAX),
            signatures: RwLock::new(None),
        }
    }
    // if the epoch id cached is the one expected, return the cached signatures, else return None
    pub(crate) async fn get_signatures(
        &self,
        expected_epoch_id: u64,
    ) -> Option<Vec<CoinIndexSignature>> {
        if self.epoch_id.load(Ordering::Relaxed) == expected_epoch_id {
            let signatures = self.signatures.read().await;
            signatures.clone()
        } else {
            None
        }
    }

    // refreshes (if needed) and returns the signatures.
    pub(crate) async fn refresh_signatures(
        &self,
        expected_epoch_id: u64,
        verification_key: &VerificationKeyAuth,
        secret_key: &SecretKeyAuth,
    ) -> Result<Vec<CoinIndexSignature>, CoconutError> {
        let mut signatures = self.signatures.write().await;

        //if this fails, it means someone else updated the signatures in the meantime
        // => We don't have to update them, and we know they exist
        // (this check can spare us some signing)
        if self.epoch_id.load(Ordering::Relaxed) != expected_epoch_id {
            *signatures = Some(sign_coin_indices(
                nym_compact_ecash::ecash_parameters(),
                verification_key,
                secret_key,
            )?);
            self.epoch_id.store(expected_epoch_id, Ordering::Relaxed);
        }

        Ok(signatures.clone().unwrap()) // Either we or someone else update the signatures, so they must be there
    }
}

pub(crate) struct ExpirationDateSignatureCache {
    pub(crate) epoch_id: AtomicU64,
    pub(crate) expiration_date: AtomicU64,
    pub(crate) signatures: RwLock<Option<Vec<ExpirationDateSignature>>>,
}

impl ExpirationDateSignatureCache {
    pub(crate) fn new() -> Self {
        ExpirationDateSignatureCache {
            epoch_id: AtomicU64::new(u64::MAX),
            expiration_date: AtomicU64::new(u64::MAX),
            signatures: RwLock::new(None),
        }
    }
    // if the epoch id cached and expiration_date cached are the ones expected, return the cached signatures, else return None
    pub(crate) async fn get_signatures(
        &self,
        expected_epoch_id: u64,
        expected_exp_date: u64,
    ) -> Option<Vec<ExpirationDateSignature>> {
        if self.epoch_id.load(Ordering::Relaxed) == expected_epoch_id
            && self.expiration_date.load(Ordering::Relaxed) == expected_exp_date
        {
            let signatures = self.signatures.read().await;
            signatures.clone()
        } else {
            None
        }
    }

    // refreshes (if needed) and returns the signatures.
    pub(crate) async fn refresh_signatures(
        &self,
        expected_epoch_id: u64,
        expected_exp_date: u64,
        secret_key: &SecretKeyAuth,
    ) -> Result<Vec<ExpirationDateSignature>, CoconutError> {
        let mut signatures = self.signatures.write().await;

        //if this fails, it means someone else updated the signatures in the meantime
        // => We don't have to update them, and we know they exist
        // (this check can spare us some signing)
        if self.epoch_id.load(Ordering::Relaxed) != expected_epoch_id
            || self.expiration_date.load(Ordering::Relaxed) != expected_exp_date
        {
            *signatures = Some(sign_expiration_date(secret_key, expected_exp_date)?);
            self.epoch_id.store(expected_epoch_id, Ordering::Relaxed);
            self.expiration_date
                .store(expected_exp_date, Ordering::Relaxed);
        }

        Ok(signatures.clone().unwrap()) // Either we or someone else update the signatures, so they must be there
    }
}
