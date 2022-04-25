// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::Addr;
use cosmwasm_std::DepsMut;
use cosmwasm_std::MessageInfo;
use cosmwasm_std::Response;
use mixnet_contract_common::events::new_settings_update_event;
use mixnet_contract_common::ContractStateParams;

pub fn try_update_rewarding_validator_address(
    deps: DepsMut<'_>,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = storage::CONTRACT_STATE.load(deps.storage)?;

    if info.sender != state.owner {
        return Err(ContractError::Unauthorized);
    }

    state.rewarding_validator_address = Addr::unchecked(address);
    storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

pub(crate) fn try_update_contract_settings(
    deps: DepsMut<'_>,
    info: MessageInfo,
    params: ContractStateParams,
) -> Result<Response, ContractError> {
    let mut state = storage::CONTRACT_STATE.load(deps.storage)?;

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized);
    }

    if params.mixnode_rewarded_set_size == 0 {
        return Err(ContractError::ZeroRewardedSet);
    }

    if params.mixnode_active_set_size == 0 {
        return Err(ContractError::ZeroActiveSet);
    }

    // note: rewarded_set = active_set + idle_set
    // hence rewarded set must always be bigger than (or equal to) the active set
    if params.mixnode_rewarded_set_size < params.mixnode_active_set_size {
        return Err(ContractError::InvalidActiveSetSize);
    }

    let response = Response::new().add_event(new_settings_update_event(&state.params, &params));

    state.params = params;
    storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(response)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{INITIAL_GATEWAY_PLEDGE, INITIAL_MIXNODE_PLEDGE};
    use crate::error::ContractError;
    use crate::mixnet_contract_settings::queries::query_rewarding_validator_address;
    use crate::mixnet_contract_settings::transactions::try_update_contract_settings;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::Response;
    use mixnet_contract_common::ContractStateParams;

    #[test]
    fn update_contract_rewarding_validtor_address() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("not-the-creator", &[]);
        let res = try_update_rewarding_validator_address(
            deps.as_mut(),
            info,
            "not-the-creator".to_string(),
        );
        assert_eq!(res, Err(ContractError::Unauthorized));

        let info = mock_info("creator", &[]);
        let res = try_update_rewarding_validator_address(
            deps.as_mut(),
            info,
            "new-good-address".to_string(),
        );
        assert_eq!(res, Ok(Response::default()));

        let state = storage::CONTRACT_STATE.load(&deps.storage).unwrap();
        assert_eq!(
            state.rewarding_validator_address,
            Addr::unchecked("new-good-address")
        );

        assert_eq!(
            state.rewarding_validator_address,
            query_rewarding_validator_address(deps.as_ref()).unwrap()
        );
    }

    #[test]
    fn updating_contract_settings() {
        let mut deps = test_helpers::init_contract();

        let new_params = ContractStateParams {
            minimum_mixnode_pledge: INITIAL_MIXNODE_PLEDGE,
            minimum_gateway_pledge: INITIAL_GATEWAY_PLEDGE,
            mixnode_rewarded_set_size: 100,
            mixnode_active_set_size: 50,
        };

        let initial_params = storage::CONTRACT_STATE
            .load(deps.as_ref().storage)
            .unwrap()
            .params;

        // sanity check to ensure new_params are different than the default ones
        assert_ne!(new_params, initial_params);

        // cannot be updated from non-owner account
        let info = mock_info("not-the-creator", &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Err(ContractError::Unauthorized));

        // but works fine from the creator account
        let info = mock_info("creator", &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(
            res,
            Ok(Response::new().add_event(new_settings_update_event(&initial_params, &new_params)))
        );

        // and the state is actually updated
        let current_state = storage::CONTRACT_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(current_state.params, new_params);

        // error is thrown if rewarded set is smaller than the active set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_rewarded_set_size = new_params.mixnode_active_set_size - 1;
        let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        assert_eq!(Err(ContractError::InvalidActiveSetSize), res);

        // error is thrown for 0 size rewarded set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_rewarded_set_size = 0;
        let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        assert_eq!(Err(ContractError::ZeroRewardedSet), res);

        // error is thrown for 0 size active set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params;
        new_params.mixnode_active_set_size = 0;
        let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        assert_eq!(Err(ContractError::ZeroActiveSet), res);
    }
}
