// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::dealings::storage::DEALINGS_BYTES;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use coconut_dkg_common::types::{ContractSafeBytes, EpochState};
use cosmwasm_std::{DepsMut, MessageInfo, Response};

pub fn try_commit_dealings(
    deps: DepsMut<'_>,
    info: MessageInfo,
    dealing_bytes: ContractSafeBytes,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::DealingExchange)?;
    // ensure the sender is a dealer
    if dealers_storage::current_dealers()
        .may_load(deps.storage, &info.sender)?
        .is_none()
    {
        return Err(ContractError::NotADealer);
    }

    // check if this dealer has already committed to all dealings
    // (we don't want to allow overwriting anything)
    for dealings in DEALINGS_BYTES {
        if !dealings.has(deps.storage, &info.sender) {
            dealings.save(deps.storage, &info.sender, &dealing_bytes)?;
            return Ok(Response::default());
        }
    }

    Err(ContractError::AlreadyCommitted {
        commitment: String::from("dealing"),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::transactions::advance_epoch_state;
    use crate::support::tests::fixtures::dealing_bytes_fixture;
    use crate::support::tests::helpers;
    use coconut_dkg_common::dealer::DealerDetails;
    use coconut_dkg_common::types::TimeConfiguration;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;

    #[test]
    fn invalid_commit_dealing() {
        let mut deps = helpers::init_contract();
        let owner = Addr::unchecked("owner");
        let mut env = mock_env();
        let info = mock_info(owner.as_str(), &[]);
        let dealing_bytes = dealing_bytes_fixture();

        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing_bytes.clone()).unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::default().to_string(),
                expected_state: EpochState::DealingExchange.to_string()
            }
        );

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        advance_epoch_state(deps.as_mut(), env).unwrap();

        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing_bytes.clone()).unwrap_err();
        assert_eq!(ret, ContractError::NotADealer);

        let dealer_details = DealerDetails {
            address: owner.clone(),
            bte_public_key_with_proof: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        dealers_storage::current_dealers()
            .save(deps.as_mut().storage, &owner, &dealer_details)
            .unwrap();

        for dealings in DEALINGS_BYTES {
            assert!(!dealings.has(deps.as_mut().storage, &owner));
            let ret = try_commit_dealings(deps.as_mut(), info.clone(), dealing_bytes.clone());
            assert!(ret.is_ok());
            assert!(dealings.has(deps.as_mut().storage, &owner));
        }
        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing_bytes.clone()).unwrap_err();
        assert_eq!(
            ret,
            ContractError::AlreadyCommitted {
                commitment: String::from("dealing"),
            }
        );
    }
}
