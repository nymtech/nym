// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::DepsMut;
use cosmwasm_std::MessageInfo;
use cosmwasm_std::Response;
use mixnet_contract::ContractStateParams;

pub(crate) fn try_update_contract_settings(
    deps: DepsMut,
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

    state.params = params;
    storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{INITIAL_GATEWAY_BOND, INITIAL_MIXNODE_BOND};
    use crate::error::ContractError;
    use crate::mixnet_contract_settings::transactions::try_update_contract_settings;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::Response;
    use mixnet_contract::ContractStateParams;

    #[test]
    fn updating_contract_settings() {
        let mut deps = test_helpers::init_contract();

        let new_params = ContractStateParams {
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_rewarded_set_size: 100,
            mixnode_active_set_size: 50,
        };

        // sanity check to ensure new_params are different than the default ones
        assert_ne!(
            new_params,
            storage::CONTRACT_STATE
                .load(deps.as_ref().storage)
                .unwrap()
                .params
        );

        // cannot be updated from non-owner account
        let info = mock_info("not-the-creator", &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Err(ContractError::Unauthorized));

        // but works fine from the creator account
        let info = mock_info("creator", &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Ok(Response::default()));

        // and the state is actually updated
        let current_state = storage::CONTRACT_STATE
            .load(deps.as_ref().storage)
            .unwrap();
        assert_eq!(current_state.params, new_params);

        // error is thrown if rewarded set is smaller than the active set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_rewarded_set_size = new_params.mixnode_active_set_size - 1;
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(Err(ContractError::InvalidActiveSetSize), res);

        // error is thrown for 0 size rewarded set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_rewarded_set_size = 0;
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(Err(ContractError::ZeroRewardedSet), res);

        // error is thrown for 0 size active set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_active_set_size = 0;
        let res = try_update_contract_settings(deps.as_mut(), info, new_params.clone());
        assert_eq!(Err(ContractError::ZeroActiveSet), res);
    }
}
