// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::mixnet_contract_settings::storage::ADMIN;
use cosmwasm_std::Response;
use cosmwasm_std::{DepsMut, StdResult};
use cosmwasm_std::{Env, MessageInfo};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_rewarding_validator_address_update_event, new_settings_update_event,
    new_update_nym_node_semver_event,
};
use mixnet_contract_common::ContractStateParamsUpdate;

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
    env: Env,
    info: MessageInfo,
    current_version: String,
) -> Result<Response, MixnetContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let new_id = storage::NymNodeVersionHistory::new().try_insert_new(
        deps.storage,
        &env,
        &current_version,
    )?;

    Ok(Response::new().add_event(new_update_nym_node_semver_event(&current_version, new_id)))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::mixnet_contract_settings::queries::query_rewarding_validator_address;
    use crate::mixnet_contract_settings::storage::rewarding_denom;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{message_info, MockApi};
    use cosmwasm_std::{Coin, Uint128};
    use cw_controllers::AdminError::NotAdmin;
    use mixnet_contract_common::OperatorsParamsUpdate;

    #[test]
    fn update_contract_rewarding_validator_address() {
        let mut deps = test_helpers::init_contract();

        let info = message_info(&deps.api.addr_make("not-the-creator"), &[]);
        let res = try_update_rewarding_validator_address(
            deps.as_mut(),
            info,
            MockApi::default().addr_make("not-the-creator").to_string(),
        );
        assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

        let info = message_info(&deps.api.addr_make("creator"), &[]);
        let res = try_update_rewarding_validator_address(
            deps.as_mut(),
            info,
            MockApi::default().addr_make("new-good-address").to_string(),
        );
        assert_eq!(
            res,
            Ok(
                Response::default().add_event(new_rewarding_validator_address_update_event(
                    MockApi::default().addr_make("rewarder"),
                    MockApi::default().addr_make("new-good-address")
                ))
            )
        );

        let state = storage::CONTRACT_STATE.load(&deps.storage).unwrap();
        assert_eq!(
            state.rewarding_validator_address,
            MockApi::default().addr_make("new-good-address")
        );

        assert_eq!(
            state.rewarding_validator_address.as_str(),
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
        let info = message_info(&deps.api.addr_make("not-the-creator"), &[]);
        let res = try_update_contract_settings(deps.as_mut(), info, pledge_update.clone());
        assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

        // but works fine from the creator account
        let info = message_info(&deps.api.addr_make("creator"), &[]);
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
        // let info = message_info("creator", &[]);
        // let mut new_params = current_state.params.clone();
        // new_params.mixnode_rewarded_set_size = new_params.mixnode_active_set_size - 1;
        // let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        // assert_eq!(Err(MixnetContractError::InvalidActiveSetSize), res);
        //
        // // error is thrown for 0 size rewarded set
        // let info = message_info("creator", &[]);
        // let mut new_params = current_state.params.clone();
        // new_params.mixnode_rewarded_set_size = 0;
        // let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        // assert_eq!(Err(MixnetContractError::ZeroRewardedSet), res);
        //
        // // error is thrown for 0 size active set
        // let info = message_info("creator", &[]);
        // let mut new_params = current_state.params;
        // new_params.mixnode_active_set_size = 0;
        // let res = try_update_contract_settings(deps.as_mut(), info, new_params);
        // assert_eq!(Err(MixnetContractError::ZeroActiveSet), res);
    }

    #[cfg(test)]
    mod updating_current_nym_node_semver {
        use super::*;
        use crate::mixnet_contract_settings::queries::query_current_nym_node_version;
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn is_restricted_to_the_admin() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let not_admin = message_info(&test.make_addr("not-admin"), &[]);
            let admin = message_info(&test.admin(), &[]);

            let env = test.env();
            let res = try_update_current_nym_node_semver(
                test.deps_mut(),
                env,
                not_admin,
                "1.2.1".to_string(),
            );
            assert!(res.is_err());

            let env = test.env();
            let res = try_update_current_nym_node_semver(
                test.deps_mut(),
                env,
                admin,
                "1.2.1".to_string(),
            );
            assert!(res.is_ok());
            Ok(())
        }

        #[test]
        fn updates_current_semver_value() -> anyhow::Result<()> {
            let mut test = TestSetup::new();

            let res = query_current_nym_node_version(test.deps())?;

            let initial = res.version.unwrap().version_information.semver;
            // sanity check to make sure our contract init hasn't changed
            assert_eq!(initial, "1.1.10");

            let update = "1.2.0".to_string();

            let env = test.env();
            let sender = test.admin_sender();
            try_update_current_nym_node_semver(test.deps_mut(), env, sender, update.clone())?;

            let updated = query_current_nym_node_version(test.deps())?;
            let version = updated.version.unwrap().version_information.semver;
            assert_eq!(version, update);

            Ok(())
        }

        #[cfg(test)]
        mod semver_chain_updates {
            use super::*;
            use crate::mixnet_contract_settings::queries::query_nym_node_version_history_paged;
            use mixnet_contract_common::{
                HistoricalNymNodeVersion, HistoricalNymNodeVersionEntry, TotalVersionDifference,
            };

            fn test_setup_with_initial_checks() -> anyhow::Result<TestSetup> {
                let test = TestSetup::new();

                let res = query_current_nym_node_version(test.deps())?;
                let initial = res.version.unwrap().version_information.semver;

                // sanity check to make sure our contract init hasn't changed
                assert_eq!(initial, "1.1.10");

                let history = query_nym_node_version_history_paged(test.deps(), None, None)?;
                assert_eq!(history.history.len(), 1);

                Ok(test)
            }

            #[test]
            fn single_patch() -> anyhow::Result<()> {
                let mut test = test_setup_with_initial_checks()?;
                let initial = query_current_nym_node_version(test.deps())?
                    .version
                    .unwrap();

                let env = test.env();
                let sender = test.admin_sender();
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender,
                    "1.1.11".to_string(),
                )?;

                let history =
                    query_nym_node_version_history_paged(test.deps(), None, None)?.history;
                assert_eq!(history.len(), 2);
                assert_eq!(history[0], initial);
                assert_eq!(
                    history[1],
                    HistoricalNymNodeVersionEntry {
                        id: 1,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.1.11".to_string(),
                            introduced_at_height: env.block.height,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 0,
                                patch: 1,
                                prerelease: 0,
                            },
                        },
                    }
                );

                Ok(())
            }

            #[test]
            fn single_minor() -> anyhow::Result<()> {
                let mut test = test_setup_with_initial_checks()?;
                let initial = query_current_nym_node_version(test.deps())?
                    .version
                    .unwrap();

                let env = test.env();
                let sender = test.admin_sender();
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender,
                    "1.2.0".to_string(),
                )?;

                let history =
                    query_nym_node_version_history_paged(test.deps(), None, None)?.history;
                assert_eq!(history.len(), 2);
                assert_eq!(history[0], initial);
                assert_eq!(
                    history[1],
                    HistoricalNymNodeVersionEntry {
                        id: 1,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.2.0".to_string(),
                            introduced_at_height: env.block.height,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 1,
                                patch: 0,
                                prerelease: 0,
                            },
                        },
                    }
                );

                Ok(())
            }

            #[test]
            fn multiple_patches() -> anyhow::Result<()> {
                let mut test = test_setup_with_initial_checks()?;
                let initial = query_current_nym_node_version(test.deps())?
                    .version
                    .unwrap();

                let mut env = test.env();
                let sender = test.admin_sender();
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.1.11".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.1.12".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender,
                    "1.1.13".to_string(),
                )?;

                let history =
                    query_nym_node_version_history_paged(test.deps(), None, None)?.history;
                assert_eq!(history.len(), 4);
                assert_eq!(history[0], initial);
                assert_eq!(
                    history[1],
                    HistoricalNymNodeVersionEntry {
                        id: 1,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.1.11".to_string(),
                            introduced_at_height: env.block.height - 2,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 0,
                                patch: 1,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[2],
                    HistoricalNymNodeVersionEntry {
                        id: 2,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.1.12".to_string(),
                            introduced_at_height: env.block.height - 1,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 0,
                                patch: 2,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[3],
                    HistoricalNymNodeVersionEntry {
                        id: 3,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.1.13".to_string(),
                            introduced_at_height: env.block.height,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 0,
                                patch: 3,
                                prerelease: 0,
                            },
                        },
                    }
                );

                Ok(())
            }

            #[test]
            fn multiple_minors() -> anyhow::Result<()> {
                let mut test = test_setup_with_initial_checks()?;
                let initial = query_current_nym_node_version(test.deps())?
                    .version
                    .unwrap();

                let mut env = test.env();
                let sender = test.admin_sender();
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.2.0".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.3.0".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender,
                    "1.4.0".to_string(),
                )?;

                let history =
                    query_nym_node_version_history_paged(test.deps(), None, None)?.history;
                assert_eq!(history.len(), 4);
                assert_eq!(history[0], initial);
                assert_eq!(
                    history[1],
                    HistoricalNymNodeVersionEntry {
                        id: 1,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.2.0".to_string(),
                            introduced_at_height: env.block.height - 2,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 1,
                                patch: 0,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[2],
                    HistoricalNymNodeVersionEntry {
                        id: 2,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.3.0".to_string(),
                            introduced_at_height: env.block.height - 1,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 2,
                                patch: 0,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[3],
                    HistoricalNymNodeVersionEntry {
                        id: 3,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.4.0".to_string(),
                            introduced_at_height: env.block.height,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 3,
                                patch: 0,
                                prerelease: 0,
                            },
                        },
                    }
                );

                Ok(())
            }

            #[test]
            fn mixed_multiple_updates() -> anyhow::Result<()> {
                let mut test = test_setup_with_initial_checks()?;
                let initial = query_current_nym_node_version(test.deps())?
                    .version
                    .unwrap();

                let mut env = test.env();
                let sender = test.admin_sender();
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.2.0".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.2.1".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    "1.2.3".to_string(),
                )?;
                env.block.height += 1;
                try_update_current_nym_node_semver(
                    test.deps_mut(),
                    env.clone(),
                    sender,
                    "1.3.0".to_string(),
                )?;

                let history =
                    query_nym_node_version_history_paged(test.deps(), None, None)?.history;
                assert_eq!(history.len(), 5);
                assert_eq!(history[0], initial);
                assert_eq!(
                    history[1],
                    HistoricalNymNodeVersionEntry {
                        id: 1,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.2.0".to_string(),
                            introduced_at_height: env.block.height - 3,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 1,
                                patch: 0,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[2],
                    HistoricalNymNodeVersionEntry {
                        id: 2,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.2.1".to_string(),
                            introduced_at_height: env.block.height - 2,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 1,
                                patch: 1,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[3],
                    HistoricalNymNodeVersionEntry {
                        id: 3,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.2.3".to_string(),
                            introduced_at_height: env.block.height - 1,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 1,
                                patch: 3,
                                prerelease: 0,
                            },
                        },
                    }
                );
                assert_eq!(
                    history[4],
                    HistoricalNymNodeVersionEntry {
                        id: 4,
                        version_information: HistoricalNymNodeVersion {
                            semver: "1.3.0".to_string(),
                            introduced_at_height: env.block.height,
                            difference_since_genesis: TotalVersionDifference {
                                major: 0,
                                minor: 2,
                                patch: 3,
                                prerelease: 0,
                            },
                        },
                    }
                );

                Ok(())
            }
        }
    }
}
