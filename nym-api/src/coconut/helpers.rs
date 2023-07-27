// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::CoconutError;
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
