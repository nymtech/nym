// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{INITIAL_GATEWAY_PLEDGE_AMOUNT, INITIAL_MIXNODE_PLEDGE_AMOUNT};
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnode_storage;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    ContractState, ContractStateParams, ExecuteMsg, InstantiateMsg, Interval, MigrateMsg,
    ProfitMarginRange, QueryMsg,
};
use nym_contracts_common::set_build_information;

// version info for migration info
const CONTRACT_NAME: &str = "crate:nym-mixnet-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn default_initial_state(
    owner: Addr,
    rewarding_validator_address: Addr,
    rewarding_denom: String,
    vesting_contract_address: Addr,
    profit_margin: ProfitMarginRange,
) -> ContractState {
    ContractState {
        owner,
        rewarding_validator_address,
        vesting_contract_address,
        rewarding_denom: rewarding_denom.clone(),
        params: ContractStateParams {
            minimum_mixnode_delegation: None,
            minimum_mixnode_pledge: Coin {
                denom: rewarding_denom.clone(),
                amount: INITIAL_MIXNODE_PLEDGE_AMOUNT,
            },
            minimum_gateway_pledge: Coin {
                denom: rewarding_denom,
                amount: INITIAL_GATEWAY_PLEDGE_AMOUNT,
            },
            profit_margin,
        },
    }
}

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, MixnetContractError> {
    if msg.epochs_in_interval == 0 {
        return Err(MixnetContractError::EpochsInIntervalZero);
    }

    if msg.epoch_duration.as_secs() == 0 {
        return Err(MixnetContractError::EpochDurationZero);
    }

    let rewarding_validator_address = deps.api.addr_validate(&msg.rewarding_validator_address)?;
    let vesting_contract_address = deps.api.addr_validate(&msg.vesting_contract_address)?;
    let state = default_initial_state(
        info.sender,
        rewarding_validator_address.clone(),
        msg.rewarding_denom,
        vesting_contract_address,
        msg.profit_margin,
    );
    let starting_interval =
        Interval::init_interval(msg.epochs_in_interval, msg.epoch_duration, &env);
    let reward_params = msg
        .initial_rewarding_params
        .into_rewarding_params(msg.epochs_in_interval)?;

    interval_storage::initialise_storage(
        deps.storage,
        starting_interval,
        rewarding_validator_address,
    )?;
    mixnet_params_storage::initialise_storage(deps.storage, state)?;
    mixnode_storage::initialise_storage(deps.storage)?;
    rewards_storage::initialise_storage(deps.storage, reward_params)?;
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, MixnetContractError> {
    match msg {
        ExecuteMsg::AssignNodeLayer { mix_id, layer } => {
            crate::mixnodes::transactions::assign_mixnode_layer(deps, info, mix_id, layer)
        }
        // families
        ExecuteMsg::CreateFamily { label } => {
            crate::families::transactions::try_create_family(deps, info, label)
        }
        ExecuteMsg::JoinFamily {
            join_permit,
            family_head,
        } => crate::families::transactions::try_join_family(deps, info, join_permit, family_head),
        ExecuteMsg::LeaveFamily { family_head } => {
            crate::families::transactions::try_leave_family(deps, info, family_head)
        }
        ExecuteMsg::KickFamilyMember { member } => {
            crate::families::transactions::try_head_kick_member(deps, info, member)
        }
        ExecuteMsg::CreateFamilyOnBehalf {
            owner_address,
            label,
        } => crate::families::transactions::try_create_family_on_behalf(
            deps,
            info,
            owner_address,
            label,
        ),
        ExecuteMsg::JoinFamilyOnBehalf {
            member_address,
            join_permit,
            family_head,
        } => crate::families::transactions::try_join_family_on_behalf(
            deps,
            info,
            member_address,
            join_permit,
            family_head,
        ),
        ExecuteMsg::LeaveFamilyOnBehalf {
            member_address,
            family_head,
        } => crate::families::transactions::try_leave_family_on_behalf(
            deps,
            info,
            member_address,
            family_head,
        ),
        ExecuteMsg::KickFamilyMemberOnBehalf {
            head_address,
            member,
        } => crate::families::transactions::try_head_kick_member_on_behalf(
            deps,
            info,
            head_address,
            member,
        ),
        // state/sys-params-related
        ExecuteMsg::UpdateRewardingValidatorAddress { address } => {
            crate::mixnet_contract_settings::transactions::try_update_rewarding_validator_address(
                deps, info, address,
            )
        }
        ExecuteMsg::UpdateContractStateParams { updated_parameters } => {
            crate::mixnet_contract_settings::transactions::try_update_contract_settings(
                deps,
                info,
                updated_parameters,
            )
        }
        ExecuteMsg::UpdateActiveSetSize {
            active_set_size,
            force_immediately,
        } => crate::rewards::transactions::try_update_active_set_size(
            deps,
            env,
            info,
            active_set_size,
            force_immediately,
        ),
        ExecuteMsg::UpdateRewardingParams {
            updated_params,
            force_immediately,
        } => crate::rewards::transactions::try_update_rewarding_params(
            deps,
            env,
            info,
            updated_params,
            force_immediately,
        ),
        ExecuteMsg::UpdateIntervalConfig {
            epochs_in_interval,
            epoch_duration_secs,
            force_immediately,
        } => crate::interval::transactions::try_update_interval_config(
            deps,
            env,
            info,
            epochs_in_interval,
            epoch_duration_secs,
            force_immediately,
        ),
        ExecuteMsg::BeginEpochTransition {} => {
            crate::interval::transactions::try_begin_epoch_transition(deps, env, info)
        }
        ExecuteMsg::AdvanceCurrentEpoch {
            new_rewarded_set,
            // families_in_layer,
            expected_active_set_size,
        } => crate::interval::transactions::try_advance_epoch(
            deps,
            env,
            info,
            new_rewarded_set,
            expected_active_set_size,
        ),
        ExecuteMsg::ReconcileEpochEvents { limit } => {
            crate::interval::transactions::try_reconcile_epoch_events(deps, env, info, limit)
        }

        // mixnode-related:
        ExecuteMsg::BondMixnode {
            mix_node,
            cost_params,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode(
            deps,
            env,
            info,
            mix_node,
            cost_params,
            owner_signature,
        ),
        ExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
            cost_params,
            owner,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_node,
            cost_params,
            owner,
            owner_signature,
        ),
        ExecuteMsg::PledgeMore {} => {
            crate::mixnodes::transactions::try_increase_pledge(deps, env, info)
        }
        ExecuteMsg::PledgeMoreOnBehalf { owner } => {
            crate::mixnodes::transactions::try_increase_pledge_on_behalf(deps, env, info, owner)
        }
        ExecuteMsg::DecreasePledge { decrease_by } => {
            crate::mixnodes::transactions::try_decrease_pledge(deps, env, info, decrease_by)
        }
        ExecuteMsg::DecreasePledgeOnBehalf { owner, decrease_by } => {
            crate::mixnodes::transactions::try_decrease_pledge_on_behalf(
                deps,
                env,
                info,
                decrease_by,
                owner,
            )
        }
        ExecuteMsg::UnbondMixnode {} => {
            crate::mixnodes::transactions::try_remove_mixnode(deps, env, info)
        }
        ExecuteMsg::UnbondMixnodeOnBehalf { owner } => {
            crate::mixnodes::transactions::try_remove_mixnode_on_behalf(deps, env, info, owner)
        }
        ExecuteMsg::UpdateMixnodeCostParams { new_costs } => {
            crate::mixnodes::transactions::try_update_mixnode_cost_params(
                deps, env, info, new_costs,
            )
        }
        ExecuteMsg::UpdateMixnodeCostParamsOnBehalf { new_costs, owner } => {
            crate::mixnodes::transactions::try_update_mixnode_cost_params_on_behalf(
                deps, env, info, new_costs, owner,
            )
        }
        ExecuteMsg::UpdateMixnodeConfig { new_config } => {
            crate::mixnodes::transactions::try_update_mixnode_config(deps, info, new_config)
        }
        ExecuteMsg::UpdateMixnodeConfigOnBehalf { new_config, owner } => {
            crate::mixnodes::transactions::try_update_mixnode_config_on_behalf(
                deps, info, new_config, owner,
            )
        }

        // gateway-related:
        ExecuteMsg::BondGateway {
            gateway,
            owner_signature,
        } => crate::gateways::transactions::try_add_gateway(
            deps,
            env,
            info,
            gateway,
            owner_signature,
        ),
        ExecuteMsg::BondGatewayOnBehalf {
            gateway,
            owner,
            owner_signature,
        } => crate::gateways::transactions::try_add_gateway_on_behalf(
            deps,
            env,
            info,
            gateway,
            owner,
            owner_signature,
        ),
        ExecuteMsg::UnbondGateway {} => {
            crate::gateways::transactions::try_remove_gateway(deps, info)
        }
        ExecuteMsg::UnbondGatewayOnBehalf { owner } => {
            crate::gateways::transactions::try_remove_gateway_on_behalf(deps, info, owner)
        }
        ExecuteMsg::UpdateGatewayConfig { new_config } => {
            crate::gateways::transactions::try_update_gateway_config(deps, info, new_config)
        }
        ExecuteMsg::UpdateGatewayConfigOnBehalf { new_config, owner } => {
            crate::gateways::transactions::try_update_gateway_config_on_behalf(
                deps, info, new_config, owner,
            )
        }

        // delegation-related:
        ExecuteMsg::DelegateToMixnode { mix_id } => {
            crate::delegations::transactions::try_delegate_to_mixnode(deps, env, info, mix_id)
        }
        ExecuteMsg::DelegateToMixnodeOnBehalf { mix_id, delegate } => {
            crate::delegations::transactions::try_delegate_to_mixnode_on_behalf(
                deps, env, info, mix_id, delegate,
            )
        }
        ExecuteMsg::UndelegateFromMixnode { mix_id } => {
            crate::delegations::transactions::try_remove_delegation_from_mixnode(
                deps, env, info, mix_id,
            )
        }
        ExecuteMsg::UndelegateFromMixnodeOnBehalf { mix_id, delegate } => {
            crate::delegations::transactions::try_remove_delegation_from_mixnode_on_behalf(
                deps, env, info, mix_id, delegate,
            )
        }

        // reward-related
        ExecuteMsg::RewardMixnode {
            mix_id,
            performance,
        } => crate::rewards::transactions::try_reward_mixnode(deps, env, info, mix_id, performance),

        ExecuteMsg::WithdrawOperatorReward {} => {
            crate::rewards::transactions::try_withdraw_operator_reward(deps, info)
        }
        ExecuteMsg::WithdrawOperatorRewardOnBehalf { owner } => {
            crate::rewards::transactions::try_withdraw_operator_reward_on_behalf(deps, info, owner)
        }
        ExecuteMsg::WithdrawDelegatorReward { mix_id } => {
            crate::rewards::transactions::try_withdraw_delegator_reward(deps, info, mix_id)
        }
        ExecuteMsg::WithdrawDelegatorRewardOnBehalf { mix_id, owner } => {
            crate::rewards::transactions::try_withdraw_delegator_reward_on_behalf(
                deps, info, mix_id, owner,
            )
        }

        // testing-only
        #[cfg(feature = "contract-testing")]
        ExecuteMsg::TestingResolveAllPendingEvents { limit } => {
            crate::testing::transactions::try_resolve_all_pending_events(deps, env, limit)
        }
    }
}

#[entry_point]
pub fn query(
    deps: Deps<'_>,
    env: Env,
    msg: QueryMsg,
) -> Result<QueryResponse, MixnetContractError> {
    let query_res = match msg {
        QueryMsg::GetAllFamiliesPaged { limit, start_after } => to_binary(
            &crate::families::queries::get_all_families_paged(deps.storage, start_after, limit)?,
        ),
        QueryMsg::GetAllMembersPaged { limit, start_after } => to_binary(
            &crate::families::queries::get_all_members_paged(deps.storage, start_after, limit)?,
        ),
        QueryMsg::GetFamilyByHead { head } => to_binary(
            &crate::families::queries::get_family_by_head(&head, deps.storage)?,
        ),
        QueryMsg::GetFamilyByLabel { label } => to_binary(
            &crate::families::queries::get_family_by_label(label, deps.storage)?,
        ),
        QueryMsg::GetFamilyMembersByHead { head } => to_binary(
            &crate::families::queries::get_family_members_by_head(&head, deps.storage)?,
        ),
        QueryMsg::GetFamilyMembersByLabel { label } => to_binary(
            &crate::families::queries::get_family_members_by_label(label, deps.storage)?,
        ),
        QueryMsg::GetContractVersion {} => {
            to_binary(&crate::mixnet_contract_settings::queries::query_contract_version())
        }
        QueryMsg::GetCW2ContractVersion {} => to_binary(&cw2::get_contract_version(deps.storage)?),
        QueryMsg::GetRewardingValidatorAddress {} => to_binary(
            &crate::mixnet_contract_settings::queries::query_rewarding_validator_address(deps)?,
        ),
        QueryMsg::GetStateParams {} => to_binary(
            &crate::mixnet_contract_settings::queries::query_contract_settings_params(deps)?,
        ),
        QueryMsg::GetState {} => {
            to_binary(&crate::mixnet_contract_settings::queries::query_contract_state(deps)?)
        }
        QueryMsg::GetRewardingParams {} => {
            to_binary(&crate::rewards::queries::query_rewarding_params(deps)?)
        }
        QueryMsg::GetEpochStatus {} => {
            to_binary(&crate::interval::queries::query_epoch_status(deps)?)
        }
        QueryMsg::GetCurrentIntervalDetails {} => to_binary(
            &crate::interval::queries::query_current_interval_details(deps, env)?,
        ),
        QueryMsg::GetRewardedSet { limit, start_after } => to_binary(
            &crate::interval::queries::query_rewarded_set_paged(deps, start_after, limit)?,
        ),

        // mixnode-related:
        QueryMsg::GetMixNodeBonds { start_after, limit } => to_binary(
            &crate::mixnodes::queries::query_mixnode_bonds_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetMixNodesDetailed { start_after, limit } => to_binary(
            &crate::mixnodes::queries::query_mixnodes_details_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetUnbondedMixNodes { limit, start_after } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnodes_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetUnbondedMixNodesByOwner {
            owner,
            limit,
            start_after,
        } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnodes_by_owner_paged(
                deps,
                owner,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetUnbondedMixNodesByIdentityKey {
            identity_key,
            limit,
            start_after,
        } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnodes_by_identity_paged(
                deps,
                identity_key,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetOwnedMixnode { address } => to_binary(
            &crate::mixnodes::queries::query_owned_mixnode(deps, address)?,
        ),
        QueryMsg::GetMixnodeDetails { mix_id } => to_binary(
            &crate::mixnodes::queries::query_mixnode_details(deps, mix_id)?,
        ),
        QueryMsg::GetMixnodeRewardingDetails { mix_id } => to_binary(
            &crate::mixnodes::queries::query_mixnode_rewarding_details(deps, mix_id)?,
        ),
        QueryMsg::GetStakeSaturation { mix_id } => to_binary(
            &crate::mixnodes::queries::query_stake_saturation(deps, mix_id)?,
        ),
        QueryMsg::GetUnbondedMixNodeInformation { mix_id } => to_binary(
            &crate::mixnodes::queries::query_unbonded_mixnode(deps, mix_id)?,
        ),
        QueryMsg::GetBondedMixnodeDetailsByIdentity { mix_identity } => to_binary(
            &crate::mixnodes::queries::query_mixnode_details_by_identity(deps, mix_identity)?,
        ),
        QueryMsg::GetLayerDistribution {} => {
            to_binary(&crate::mixnodes::queries::query_layer_distribution(deps)?)
        }

        // gateway-related:
        QueryMsg::GetGateways { limit, start_after } => to_binary(
            &crate::gateways::queries::query_gateways_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetGatewayBond { identity } => to_binary(
            &crate::gateways::queries::query_gateway_bond(deps, identity)?,
        ),
        QueryMsg::GetOwnedGateway { address } => to_binary(
            &crate::gateways::queries::query_owned_gateway(deps, address)?,
        ),

        // delegation-related:
        QueryMsg::GetMixnodeDelegations {
            mix_id,
            start_after,
            limit,
        } => to_binary(
            &crate::delegations::queries::query_mixnode_delegations_paged(
                deps,
                mix_id,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetDelegatorDelegations {
            delegator,
            start_after,
            limit,
        } => to_binary(
            &crate::delegations::queries::query_delegator_delegations_paged(
                deps,
                delegator,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetDelegationDetails {
            mix_id,
            delegator,
            proxy,
        } => to_binary(&crate::delegations::queries::query_mixnode_delegation(
            deps, mix_id, delegator, proxy,
        )?),
        QueryMsg::GetAllDelegations { start_after, limit } => to_binary(
            &crate::delegations::queries::query_all_delegations_paged(deps, start_after, limit)?,
        ),

        // rewards related
        QueryMsg::GetPendingOperatorReward { address } => to_binary(
            &crate::rewards::queries::query_pending_operator_reward(deps, address)?,
        ),
        QueryMsg::GetPendingMixNodeOperatorReward { mix_id } => to_binary(
            &crate::rewards::queries::query_pending_mixnode_operator_reward(deps, mix_id)?,
        ),
        QueryMsg::GetPendingDelegatorReward {
            address,
            mix_id,
            proxy,
        } => to_binary(&crate::rewards::queries::query_pending_delegator_reward(
            deps, address, mix_id, proxy,
        )?),
        QueryMsg::GetEstimatedCurrentEpochOperatorReward {
            mix_id,
            estimated_performance,
        } => to_binary(
            &crate::rewards::queries::query_estimated_current_epoch_operator_reward(
                deps,
                mix_id,
                estimated_performance,
            )?,
        ),
        QueryMsg::GetEstimatedCurrentEpochDelegatorReward {
            address,
            mix_id,
            proxy,
            estimated_performance,
        } => to_binary(
            &crate::rewards::queries::query_estimated_current_epoch_delegator_reward(
                deps,
                address,
                mix_id,
                proxy,
                estimated_performance,
            )?,
        ),

        // interval-related
        QueryMsg::GetPendingEpochEvents { limit, start_after } => {
            to_binary(&crate::interval::queries::query_pending_epoch_events_paged(
                deps,
                env,
                start_after,
                limit,
            )?)
        }
        QueryMsg::GetPendingIntervalEvents { limit, start_after } => to_binary(
            &crate::interval::queries::query_pending_interval_events_paged(
                deps,
                env,
                start_after,
                limit,
            )?,
        ),
        QueryMsg::GetPendingEpochEvent { event_id } => to_binary(
            &crate::interval::queries::query_pending_epoch_event(deps, event_id)?,
        ),
        QueryMsg::GetPendingIntervalEvent { event_id } => to_binary(
            &crate::interval::queries::query_pending_interval_event(deps, event_id)?,
        ),
        QueryMsg::GetNumberOfPendingEvents {} => to_binary(
            &crate::interval::queries::query_number_of_pending_events(deps)?,
        ),

        // signing-related
        QueryMsg::GetSigningNonce { address } => to_binary(
            &crate::signing::queries::query_current_signing_nonce(deps, address)?,
        ),
    };

    Ok(query_res?)
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    msg: MigrateMsg,
) -> Result<Response, MixnetContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // due to circular dependency on contract addresses (i.e. mixnet contract requiring vesting contract address
    // and vesting contract requiring the mixnet contract address), if we ever want to deploy any new fresh
    // environment, one of the contracts will HAVE TO go through a migration
    if let Some(vesting_contract_address) = msg.vesting_contract_address {
        let mut current_state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
        current_state.vesting_contract_address =
            deps.api.addr_validate(&vesting_contract_address)?;
        mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &current_state)?;
    }

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Decimal;
    use mixnet_contract_common::reward_params::{IntervalRewardParams, RewardingParams};
    use mixnet_contract_common::{InitialRewardingParams, Percent};
    use std::time::Duration;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let init_msg = InstantiateMsg {
            rewarding_validator_address: "foomp123".to_string(),
            vesting_contract_address: "bar456".to_string(),
            rewarding_denom: "uatom".to_string(),
            epochs_in_interval: 1234,
            epoch_duration: Duration::from_secs(4321),
            initial_rewarding_params: InitialRewardingParams {
                initial_reward_pool: Decimal::from_atomics(100_000_000_000_000u128, 0).unwrap(),
                initial_staking_supply: Decimal::from_atomics(123_456_000_000_000u128, 0).unwrap(),
                staking_supply_scale_factor: Percent::hundred(),
                sybil_resistance: Percent::from_percentage_value(23).unwrap(),
                active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                interval_pool_emission: Percent::from_percentage_value(1).unwrap(),
                rewarded_set_size: 543,
                active_set_size: 123,
            },
            profit_margin: ProfitMarginRange {
                minimum: "0.05".parse().unwrap(),
                maximum: "0.95".parse().unwrap(),
            },
        };

        let sender = mock_info("sender", &[]);
        let res = instantiate(deps.as_mut(), env, sender, init_msg);
        assert!(res.is_ok());

        let expected_state = ContractState {
            owner: Addr::unchecked("sender"),
            rewarding_validator_address: Addr::unchecked("foomp123"),
            vesting_contract_address: Addr::unchecked("bar456"),
            rewarding_denom: "uatom".into(),
            params: ContractStateParams {
                minimum_mixnode_delegation: None,
                minimum_mixnode_pledge: Coin {
                    denom: "uatom".into(),
                    amount: INITIAL_MIXNODE_PLEDGE_AMOUNT,
                },
                minimum_gateway_pledge: Coin {
                    denom: "uatom".into(),
                    amount: INITIAL_GATEWAY_PLEDGE_AMOUNT,
                },
                profit_margin: ProfitMarginRange {
                    minimum: Percent::from_percentage_value(5).unwrap(),
                    maximum: Percent::from_percentage_value(95).unwrap(),
                },
            },
        };

        let expected_epoch_reward_budget =
            Decimal::from_ratio(100_000_000_000_000u128, 1234u32) * Decimal::percent(1);
        let expected_stake_saturation_point = Decimal::from_ratio(123_456_000_000_000u128, 543u32);

        let expected_rewarding_params = RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: Decimal::from_atomics(100_000_000_000_000u128, 0).unwrap(),
                staking_supply: Decimal::from_atomics(123_456_000_000_000u128, 0).unwrap(),
                staking_supply_scale_factor: Percent::hundred(),
                epoch_reward_budget: expected_epoch_reward_budget,
                stake_saturation_point: expected_stake_saturation_point,
                sybil_resistance: Percent::from_percentage_value(23).unwrap(),
                active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                interval_pool_emission: Percent::from_percentage_value(1).unwrap(),
            },
            rewarded_set_size: 543,
            active_set_size: 123,
        };

        let state = mixnet_params_storage::CONTRACT_STATE
            .load(deps.as_ref().storage)
            .unwrap();
        assert_eq!(state, expected_state);

        let rewarding_params = rewards_storage::REWARDING_PARAMS
            .load(deps.as_ref().storage)
            .unwrap();
        assert_eq!(rewarding_params, expected_rewarding_params);

        let interval = interval_storage::current_interval(deps.as_ref().storage).unwrap();
        assert_eq!(interval.epochs_in_interval(), 1234);
        assert_eq!(interval.epoch_length(), Duration::from_secs(4321));
        assert_eq!(interval.current_interval_id(), 0);
        assert_eq!(interval.current_epoch_id(), 0);
    }
}
