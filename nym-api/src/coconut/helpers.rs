// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::CoconutError;
use crate::coconut::state::bandwidth_credential_params;
use nym_api_requests::coconut::models::FreePassRequest;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut::{Attribute, BlindSignRequest, BlindedSignature, SecretKey};
use nym_validator_client::nyxd::error::NyxdError::AbciError;

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
    fn blind_sign_request(&self) -> &BlindSignRequest;

    fn public_attributes(&self) -> Vec<Attribute>;
}

impl CredentialRequest for BlindSignRequestBody {
    fn blind_sign_request(&self) -> &BlindSignRequest {
        &self.inner_sign_request
    }

    fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes_hashed()
    }
}

impl CredentialRequest for FreePassRequest {
    fn blind_sign_request(&self) -> &BlindSignRequest {
        &self.inner_sign_request
    }

    fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes_hashed()
    }
}

pub(crate) fn blind_sign<C: CredentialRequest>(
    request: &C,
    signing_key: &SecretKey,
) -> Result<BlindedSignature, CoconutError> {
    let public_attributes = request.public_attributes();
    let attributes_ref = public_attributes.iter().collect::<Vec<_>>();

    Ok(nym_coconut::blind_sign(
        bandwidth_credential_params(),
        signing_key,
        &request.blind_sign_request(),
        &attributes_ref,
    )?)
}
