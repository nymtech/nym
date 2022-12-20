// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::current_dealers;
use crate::epoch_state::storage::{CURRENT_EPOCH_STATE, THRESHOLD};
use crate::error::ContractError;
use crate::state::ADMIN;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::{DepsMut, MessageInfo, Order, Response};

pub(crate) fn advance_epoch_state(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch_state =
        CURRENT_EPOCH_STATE.update::<_, ContractError>(deps.storage, |mut epoch_state| {
            // TODO: When defaulting to the first state, some action will probably need to be taken on the
            // rest of the contract, as we're starting with a new set of signers
            epoch_state = epoch_state.next().unwrap_or_default();
            Ok(epoch_state)
        })?;
    if current_epoch_state == EpochState::DealingExchange {
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
    use coconut_dkg_common::types::{DealerDetails, EpochState};
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::Addr;
    use cw_controllers::AdminError;

    #[test]
    fn advance_state() {
        let mut deps = init_contract();
        let info = mock_info("requester", &[]);
        let admin_info = mock_info(ADMIN_ADDRESS, &[]);

        assert_eq!(
            advance_epoch_state(deps.as_mut(), info).unwrap_err(),
            ContractError::Admin(AdminError::NotAdmin {})
        );

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            CURRENT_EPOCH_STATE.load(deps.as_mut().storage).unwrap(),
            EpochState::DealingExchange
        );

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            CURRENT_EPOCH_STATE.load(deps.as_mut().storage).unwrap(),
            EpochState::VerificationKeySubmission
        );

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            CURRENT_EPOCH_STATE.load(deps.as_mut().storage).unwrap(),
            EpochState::VerificationKeyValidation
        );

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            CURRENT_EPOCH_STATE.load(deps.as_mut().storage).unwrap(),
            EpochState::VerificationKeyFinalization
        );

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            CURRENT_EPOCH_STATE.load(deps.as_mut().storage).unwrap(),
            EpochState::InProgress
        );

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            CURRENT_EPOCH_STATE.load(deps.as_mut().storage).unwrap(),
            EpochState::PublicKeySubmission
        );
    }

    #[test]
    fn verify_threshold() {
        let mut deps = init_contract();
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

        advance_epoch_state(deps.as_mut(), admin_info.clone()).unwrap();
        assert_eq!(
            THRESHOLD.may_load(deps.as_mut().storage).unwrap().unwrap(),
            67
        );
    }
}
