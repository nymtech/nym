// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::verification_key_shares::storage::VK_SHARES;
use coconut_dkg_common::types::EpochState;
use coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use cosmwasm_std::{DepsMut, MessageInfo, Response};

pub fn try_commit_verification_key_share(
    deps: DepsMut<'_>,
    info: MessageInfo,
    share: VerificationKeyShare,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::VerificationKeySubmission)?;
    // ensure the sender is a dealer
    let details = dealers_storage::current_dealers()
        .load(deps.storage, &info.sender)
        .map_err(|_| ContractError::NotADealer)?;
    if VK_SHARES.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::AlreadyCommitted {
            commitment: String::from("verification key share"),
        });
    }

    let data = ContractVKShare {
        share,
        node_index: details.assigned_index,
        owner: info.sender.clone(),
    };
    VK_SHARES.save(deps.storage, &info.sender, &data)?;

    Ok(Response::default())
}
