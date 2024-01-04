// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::dealings::storage::StoredDealing;
use crate::epoch_state::storage::{CURRENT_EPOCH, INITIAL_REPLACEMENT_DATA};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::STATE;
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use nym_coconut_dkg_common::types::{EpochState, PartialContractDealing};

pub fn try_commit_dealings(
    deps: DepsMut<'_>,
    info: MessageInfo,
    dealing: PartialContractDealing,
    resharing: bool,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::DealingExchange { resharing })?;
    // ensure the sender is a dealer
    if dealers_storage::current_dealers()
        .may_load(deps.storage, &info.sender)?
        .is_none()
    {
        return Err(ContractError::NotADealer);
    }
    if resharing
        && !INITIAL_REPLACEMENT_DATA
            .load(deps.storage)?
            .initial_dealers
            .contains(&info.sender)
    {
        return Err(ContractError::NotAnInitialDealer);
    }

    let state = STATE.load(deps.storage)?;
    let epoch = CURRENT_EPOCH.load(deps.storage)?;

    // check if the index is in range without doing expensive storage reads
    // note: dealing indexing starts from 0
    if dealing.index >= state.key_size {
        return Err(ContractError::DealingOutOfRange {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            index: dealing.index,
            key_size: state.key_size,
        });
    }

    // check if this dealer has already committed this particular dealing
    if StoredDealing::exists(deps.storage, epoch.epoch_id, &info.sender, dealing.index) {
        return Err(ContractError::DealingAlreadyCommitted {
            epoch_id: epoch.epoch_id,
            dealer: info.sender,
            index: dealing.index,
        });
    }

    StoredDealing::save(deps.storage, epoch.epoch_id, &info.sender, dealing);

    Ok(Response::new())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::storage::CURRENT_EPOCH;
    use crate::epoch_state::transactions::advance_epoch_state;
    use crate::support::tests::fixtures::{dealer_details_fixture, partial_dealing_fixture};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::add_fixture_dealer;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::{
        ContractSafeBytes, InitialReplacementData, TimeConfiguration, TOTAL_DEALINGS,
    };

    #[test]
    fn invalid_commit_dealing() {
        let mut deps = helpers::init_contract();
        let owner = Addr::unchecked("owner1");
        let mut env = mock_env();
        let info = mock_info(owner.as_str(), &[]);
        let dealing = partial_dealing_fixture();

        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing.clone(), false).unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::default().to_string(),
                expected_state: EpochState::DealingExchange { resharing: false }.to_string()
            }
        );

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        add_fixture_dealer(deps.as_mut());
        advance_epoch_state(deps.as_mut(), env).unwrap();

        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing.clone(), false).unwrap_err();
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

        // assume we're in resharing mode
        CURRENT_EPOCH
            .update::<_, ContractError>(deps.as_mut().storage, |mut epoch| {
                epoch.state = EpochState::DealingExchange { resharing: true };
                Ok(epoch)
            })
            .unwrap();
        INITIAL_REPLACEMENT_DATA
            .save(
                deps.as_mut().storage,
                &InitialReplacementData {
                    initial_dealers: vec![],
                    initial_height: 1,
                },
            )
            .unwrap();
        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing.clone(), true).unwrap_err();
        assert_eq!(ret, ContractError::NotAnInitialDealer);

        INITIAL_REPLACEMENT_DATA
            .update::<_, ContractError>(deps.as_mut().storage, |mut data| {
                data.initial_dealers = vec![dealer_details_fixture(1).address];
                Ok(data)
            })
            .unwrap();

        // back to 'normal' mode
        CURRENT_EPOCH
            .update::<_, ContractError>(deps.as_mut().storage, |mut epoch| {
                epoch.state = EpochState::DealingExchange { resharing: false };
                Ok(epoch)
            })
            .unwrap();

        // dealing out of range
        let ret = try_commit_dealings(
            deps.as_mut(),
            info.clone(),
            PartialContractDealing {
                index: 42,
                data: ContractSafeBytes(vec![1, 2, 3]),
            },
            false,
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::DealingOutOfRange {
                epoch_id: 0,
                dealer: info.sender.clone(),
                index: 42,
                key_size: TOTAL_DEALINGS as u32,
            }
        );

        // 'good' dealing
        let ret = try_commit_dealings(deps.as_mut(), info.clone(), dealing.clone(), false);
        assert!(ret.is_ok());

        // duplicate dealing
        let ret =
            try_commit_dealings(deps.as_mut(), info.clone(), dealing.clone(), false).unwrap_err();
        assert_eq!(
            ret,
            ContractError::DealingAlreadyCommitted {
                epoch_id: 0,
                dealer: info.sender.clone(),
                index: 0,
            }
        );

        // same index, but next epoch
        CURRENT_EPOCH
            .update::<_, ContractError>(deps.as_mut().storage, |mut epoch| {
                epoch.epoch_id += 1;
                Ok(epoch)
            })
            .unwrap();

        let ret = try_commit_dealings(deps.as_mut(), info.clone(), dealing.clone(), false);
        assert!(ret.is_ok());
    }
}
