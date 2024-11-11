// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::mixnet_contract_settings::storage::ADMIN;
use cosmwasm_std::MessageInfo;
use cosmwasm_std::Response;
use cosmwasm_std::{DepsMut, StdResult};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_rewarding_validator_address_update_event, new_settings_update_event,
    new_update_nym_node_semver_event,
};
use mixnet_contract_common::ContractStateParamsUpdate;
use std::str::FromStr;

pub fn try_update_contract_admin(
    mut deps: DepsMut<'_>,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, MixnetContractError> {
    let new_admin = deps.api.addr_validate(&new_admin)?;

    let res = ADMIN.execute_update_admin(deps.branch(), info, Some(new_admin.clone()))?;

    // SAFETY: we don't need to perform any authentication checks on the sender as it was performed
    // during 'execute_update_admin' call
    #[allow(deprecated)]
    storage::CONTRACT_STATE.update(deps.storage, |mut state| -> StdResult<_> {
        state.owner = Some(new_admin);
        Ok(state)
    })?;

    Ok(res)
}

pub fn try_update_rewarding_validator_address(
    deps: DepsMut<'_>,
    info: MessageInfo,
    address: String,
) -> Result<Response, MixnetContractError> {
    let mut state = storage::CONTRACT_STATE.load(deps.storage)?;

    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let new_address = deps.api.addr_validate(&address)?;
    let old_address = state.rewarding_validator_address;

    state.rewarding_validator_address = new_address.clone();
    storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(
        Response::new().add_event(new_rewarding_validator_address_update_event(
            old_address,
            new_address,
        )),
    )
}

pub(crate) fn try_update_contract_settings(
    deps: DepsMut<'_>,
    info: MessageInfo,
    update: ContractStateParamsUpdate,
) -> Result<Response, MixnetContractError> {
    let mut state = storage::CONTRACT_STATE.load(deps.storage)?;
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    if !update.contains_updates() {
        return Err(MixnetContractError::EmptyStateUpdateMsg);
    }

    let response = Response::new().add_event(new_settings_update_event(&update));

    // check for delegations params updates
    if let Some(delegations_update) = update.delegations_params {
        // there's only a single field there to change
        state.params.delegations_params.minimum_delegation = delegations_update.minimum_delegation;
    }

    // check for operators params updates
    if let Some(operators_update) = update.operators_params {
        if let Some(minimum_pledge) = operators_update.minimum_pledge {
            state.params.operators_params.minimum_pledge = minimum_pledge
        }
        if let Some(profit_margin) = operators_update.profit_margin {
            state.params.operators_params.profit_margin = profit_margin
        }
        if let Some(interval_operating_cost) = operators_update.interval_operating_cost {
            state.params.operators_params.interval_operating_cost = interval_operating_cost;
        }
    }

    // check for config score params updates
    if let Some(config_score_update) = update.config_score_params {
        // if semver is to be updated - validate the provided value
        if let Some(current_nym_node_semver) = config_score_update.current_nym_node_semver {
            if semver::Version::from_str(&current_nym_node_semver).is_err() {
                return Err(MixnetContractError::InvalidNymNodeSemver {
                    provided: current_nym_node_semver,
                });
            }
            state.params.config_score_params.current_nym_node_semver = current_nym_node_semver
        }
        if let Some(version_weights) = config_score_update.version_weights {
            state.params.config_score_params.version_weights = version_weights
        }
        if let Some(version_score_formula_params) = config_score_update.version_score_formula_params
        {
            state
                .params
                .config_score_params
                .version_score_formula_params = version_score_formula_params
        }
    }

    storage::CONTRACT_STATE.save(deps.storage, &state)?;
    Ok(response)
}

pub(crate) fn try_update_current_nym_node_semver(
    deps: DepsMut<'_>,
    info: MessageInfo,
    current_version: String,
) -> Result<Response, MixnetContractError> {
    let mut state = storage::CONTRACT_STATE.load(deps.storage)?;
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let response = Response::new().add_event(new_update_nym_node_semver_event(&current_version));

    if semver::Version::from_str(&current_version).is_err() {
        return Err(MixnetContractError::InvalidNymNodeSemver {
            provided: current_version,
        });
    }

    state.params.config_score_params.current_nym_node_semver = current_version;
    storage::CONTRACT_STATE.save(deps.storage, &state)?;
    Ok(response)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::mixnet_contract_settings::queries::query_rewarding_validator_address;
    use crate::mixnet_contract_settings::storage::rewarding_denom;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{Addr, Coin, Uint128};
    use cw_controllers::AdminError::NotAdmin;
    use mixnet_contract_common::OperatorsParamsUpdate;

    #[test]
    fn update_contract_rewarding_validator_address() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("not-the-creator", &[]);
        let res = try_update_rewarding_validator_address(
            deps.as_mut(),
            info,
            "not-the-creator".to_string(),
        );
        assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

        let info = mock_info("creator", &[]);
        let res = try_update_rewarding_validator_address(
            deps.as_mut(),
            info,
            "new-good-address".to_string(),
        );
        assert_eq!(
            res,
            Ok(
                Response::default().add_event(new_rewarding_validator_address_update_event(
                    Addr::unchecked("rewarder"),
                    Addr::unchecked("new-good-address")
                ))
            )
        );

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
        let denom = rewarding_denom(deps.as_ref().storage).unwrap();

        let pledge_update = ContractStateParamsUpdate {
            delegations_params: None,
            operators_params: Some(OperatorsParamsUpdate {
                minimum_pledge: Some(Coin {
                    denom: denom.clone(),
                    amount: Uint128::new(12345),
                }),
                profit_margin: None,
                interval_operating_cost: None,
            }),
            config_score_params: None,
        };

        // cannot be updated from non-owner account
        let info = mock_info("not-the-creator", &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, pledge_update.clone());
        assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

        // but works fine from the creator account
        let info = mock_info("creator", &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, pledge_update.clone());
        assert_eq!(
            res,
            Ok(Response::new().add_event(new_settings_update_event(&pledge_update)))
        );

        // and the state is actually updated
        let current_state = storage::CONTRACT_STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            current_state.params.operators_params.minimum_pledge,
            pledge_update
                .operators_params
                .unwrap()
                .minimum_pledge
                .unwrap()
        );

        // // error is thrown if rewarded set is smaller than the active set
        // let info = mock_info("creator", &[]);
        // let mut new_params = current_state.params.clone();
        // new_params.mixnode_rewarded_set_size = new_params.mixnode_active_set_size - 1;
        // let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        // assert_eq!(Err(MixnetContractError::InvalidActiveSetSize), res);
        //
        // // error is thrown for 0 size rewarded set
        // let info = mock_info("creator", &[]);
        // let mut new_params = current_state.params.clone();
        // new_params.mixnode_rewarded_set_size = 0;
        // let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        // assert_eq!(Err(MixnetContractError::ZeroRewardedSet), res);
        //
        // // error is thrown for 0 size active set
        // let info = mock_info("creator", &[]);
        // let mut new_params = current_state.params;
        // new_params.mixnode_active_set_size = 0;
        // let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        // assert_eq!(Err(MixnetContractError::ZeroActiveSet), res);
    }
}
