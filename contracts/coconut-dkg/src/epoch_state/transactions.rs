// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::current_dealers;
use crate::epoch_state::storage::{CURRENT_EPOCH, THRESHOLD};
use crate::error::ContractError;
use crate::state::ADMIN;
use coconut_dkg_common::types::{Epoch, EpochState};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Order, Response};

pub(crate) fn advance_epoch_state(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = CURRENT_EPOCH.update::<_, ContractError>(deps.storage, |mut epoch| {
        // TODO: When defaulting to the first state, some action will probably need to be taken on the
        // rest of the contract, as we're starting with a new set of signers
        epoch = Epoch::new(epoch.state.next().unwrap_or_default(), env.block.time);
        Ok(epoch)
    })?;
    if current_epoch.state == EpochState::DealingExchange {
        let current_dealer_count = current_dealers()
            .keys(deps.storage, None, None, Order::Ascending)
            .count();
        // note: ceiling in integer division can be achieved via q = (x + y - 1) / y;
        let threshold = (2 * current_dealer_count as u64 + 3 - 1) / 3;
        THRESHOLD.save(deps.storage, &threshold)?;
    }
    Ok(Response::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use coconut_dkg_common::types::{
        DealerDetails, EpochState, DEALING_EXCHANGE_TIME_SECS, IN_PROGRESS_TIME_SECS,
        PUBLIC_KEY_SUBMISSION_TIME_SECS, VERIFICATION_KEY_FINALIZATION_TIME_SECS,
        VERIFICATION_KEY_SUBMISSION_TIME_SECS, VERIFICATION_KEY_VALIDATION_TIME_SECS,
    };
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;
    use cw_controllers::AdminError;

    #[test]
    fn advance_state() {
        let mut deps = init_contract();
        let env = mock_env();
        let info = mock_info("requester", &[]);
        let admin_info = mock_info(ADMIN_ADDRESS, &[]);

        assert_eq!(
            advance_epoch_state(deps.as_mut(), env.clone(), info).unwrap_err(),
            ContractError::Admin(AdminError::NotAdmin {})
        );

        advance_epoch_state(deps.as_mut(), env.clone(), admin_info.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::DealingExchange);
        assert_eq!(
            epoch.finish_timestamp,
            env.block.time.plus_seconds(DEALING_EXCHANGE_TIME_SECS)
        );

        advance_epoch_state(deps.as_mut(), env.clone(), admin_info.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::VerificationKeySubmission);
        assert_eq!(
            epoch.finish_timestamp,
            env.block
                .time
                .plus_seconds(VERIFICATION_KEY_SUBMISSION_TIME_SECS)
        );

        advance_epoch_state(deps.as_mut(), env.clone(), admin_info.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::VerificationKeyValidation);
        assert_eq!(
            epoch.finish_timestamp,
            env.block
                .time
                .plus_seconds(VERIFICATION_KEY_VALIDATION_TIME_SECS)
        );

        advance_epoch_state(deps.as_mut(), env.clone(), admin_info.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::VerificationKeyFinalization);
        assert_eq!(
            epoch.finish_timestamp,
            env.block
                .time
                .plus_seconds(VERIFICATION_KEY_FINALIZATION_TIME_SECS)
        );

        advance_epoch_state(deps.as_mut(), env.clone(), admin_info.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::InProgress);
        assert_eq!(
            epoch.finish_timestamp,
            env.block.time.plus_seconds(IN_PROGRESS_TIME_SECS)
        );

        advance_epoch_state(deps.as_mut(), env.clone(), admin_info.clone()).unwrap();
        let epoch = CURRENT_EPOCH.load(deps.as_mut().storage).unwrap();
        assert_eq!(epoch.state, EpochState::PublicKeySubmission);
        assert_eq!(
            epoch.finish_timestamp,
            env.block.time.plus_seconds(PUBLIC_KEY_SUBMISSION_TIME_SECS)
        );
    }

    #[test]
    fn verify_threshold() {
        let mut deps = init_contract();
        let env = mock_env();
        let admin_info = mock_info(ADMIN_ADDRESS, &[]);

        assert!(THRESHOLD.may_load(deps.as_mut().storage).unwrap().is_none());

        for i in 1..101 {
            let address = Addr::unchecked(format!("dealer{}", i));
            current_dealers()
                .save(
                    deps.as_mut().storage,
                    &address,
                    &DealerDetails {
                        address: address.clone(),
                        bte_public_key_with_proof: "bte_public_key_with_proof".to_string(),
                        announce_address: "127.0.0.1".to_string(),
                        assigned_index: i,
                    },
                )
                .unwrap();
        }

        advance_epoch_state(deps.as_mut(), env, admin_info.clone()).unwrap();
        assert_eq!(
            THRESHOLD.may_load(deps.as_mut().storage).unwrap().unwrap(),
            67
        );
    }
}
