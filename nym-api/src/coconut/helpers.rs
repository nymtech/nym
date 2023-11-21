// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::CoconutError;
use crate::coconut::InternalSignRequest;
use nym_coconut::{BlindedSignature, Parameters};
use nym_coconut_interface::KeyPair as CoconutKeyPair;
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
    request: InternalSignRequest,
    key_pair: &CoconutKeyPair,
) -> Result<BlindedSignature, CoconutError> {
    let params = Parameters::new(request.total_params())?;
    Ok(nym_coconut_interface::blind_sign(
        &params,
        &key_pair.secret_key(),
        request.blind_sign_request(),
        request.public_attributes(),
    )?)
}
