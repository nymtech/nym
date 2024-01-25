// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::CoconutError;
use crate::coconut::state::bandwidth_voucher_params;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut::{BlindedSignature, SecretKey};
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

pub(crate) fn blind_sign(
    request: &BlindSignRequestBody,
    signing_key: &SecretKey,
) -> Result<BlindedSignature, CoconutError> {
    let public_attributes = request.public_attributes_hashed();
    let attributes_ref = public_attributes.iter().collect::<Vec<_>>();

    Ok(nym_coconut_interface::blind_sign(
        bandwidth_voucher_params(),
        signing_key,
        &request.inner_sign_request,
        &attributes_ref,
    )?)
}
