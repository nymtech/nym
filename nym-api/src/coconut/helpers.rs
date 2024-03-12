// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::atomic::{AtomicU64, Ordering};

use crate::coconut::error::CoconutError;
use crate::coconut::state::bandwidth_credential_params;
use nym_api_requests::coconut::models::FreePassRequest;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_compact_ecash::scheme::expiration_date_signatures::{
    sign_expiration_date, ExpirationDateSignature,
};
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::setup::{sign_coin_indices, CoinIndexSignature, Parameters};
use nym_compact_ecash::utils::BlindedSignature;
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
    fn expiration_date(&self) -> u64;
    fn ecash_pubkey(&self) -> PublicKeyUser;
}

impl CredentialRequest for BlindSignRequestBody {
    fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.inner_sign_request
    }

    fn expiration_date(&self) -> u64 {
        self.expiration_date
    }

    fn ecash_pubkey(&self) -> PublicKeyUser {
        self.ecash_pubkey.clone()
    }
}

impl CredentialRequest for FreePassRequest {
    fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.inner_sign_request
    }

    fn expiration_date(&self) -> u64 {
        self.expiration_date
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
        bandwidth_credential_params().grp(),
        signing_key.clone(),
        request.ecash_pubkey().clone(),
        request.withdrawal_request(),
        request.expiration_date(),
    )?)
}
}
