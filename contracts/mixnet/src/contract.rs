// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::INITIAL_PLEDGE_AMOUNT;
use crate::interval::storage as interval_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::nodes::storage as nymnodes_storage;
use crate::queued_migrations::migrate_to_nym_nodes_usage;
use crate::rewards::storage::RewardingStorage;
use cosmwasm_std::{
    coin, entry_point, to_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response,
};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    ContractState, ContractStateParams, ExecuteMsg, InstantiateMsg, Interval, MigrateMsg,
    NodeCostParams, OperatingCostRange, ProfitMarginRange, QueryMsg,
};
use nym_contracts_common::{set_build_information, Percent};

// version info for migration info
const CONTRACT_NAME: &str = "crate:nym-mixnet-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn default_initial_state(
    owner: Addr,
    rewarding_validator_address: Addr,
    rewarding_denom: String,
    vesting_contract_address: Addr,
    profit_margin: ProfitMarginRange,
    interval_operating_cost: OperatingCostRange,
) -> ContractState {
    // we have to temporarily preserve this functionalities until it can be removed
    #[allow(deprecated)]
    ContractState {
        owner: Some(owner),
        rewarding_validator_address,
        vesting_contract_address,
        rewarding_denom: rewarding_denom.clone(),
        params: ContractStateParams {
            minimum_delegation: None,
            minimum_pledge: Coin {
                denom: rewarding_denom.clone(),
                amount: INITIAL_PLEDGE_AMOUNT,
            },
            profit_margin,
            interval_operating_cost,
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
    mut deps: DepsMut<'_>,
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
        info.sender.clone(),
        rewarding_validator_address.clone(),
        msg.rewarding_denom,
        vesting_contract_address,
        msg.profit_margin,
        msg.interval_operating_cost,
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
    mixnet_params_storage::initialise_storage(deps.branch(), state, info.sender)?;
    RewardingStorage::new().initialise(deps.storage, reward_params)?;
    nymnodes_storage::initialise_storage(deps.storage)?;
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
        // state/sys-params-related
        ExecuteMsg::UpdateAdmin { admin } => {
            crate::mixnet_contract_settings::transactions::try_update_contract_admin(
                deps, info, admin,
            )
        }
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
        ExecuteMsg::UpdateActiveSetDistribution {
            update,
            force_immediately,
        } => crate::rewards::transactions::try_update_active_set_distribution(
            deps,
            env,
            info,
            update,
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
        ExecuteMsg::AssignRoles { assignment } => {
            crate::interval::transactions::try_assign_roles(deps, env, info, assignment)
        }
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
        ExecuteMsg::UnbondMixnode {} => {
            crate::mixnodes::transactions::try_remove_mixnode(deps, env, info)
        }
        ExecuteMsg::UpdateMixnodeConfig { new_config } => {
            crate::mixnodes::transactions::try_update_mixnode_config(deps, info, new_config)
        }
        ExecuteMsg::MigrateMixnode {} => {
            crate::mixnodes::transactions::try_migrate_to_nymnode(deps, info)
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
        ExecuteMsg::UnbondGateway {} => {
            crate::gateways::transactions::try_remove_gateway(deps, info)
        }
        ExecuteMsg::UpdateGatewayConfig { new_config } => {
            crate::gateways::transactions::try_update_gateway_config(deps, info, new_config)
        }
        ExecuteMsg::MigrateGateway { cost_params } => {
            crate::gateways::transactions::try_migrate_to_nymnode(deps, info, cost_params)
        }

        // nym-node related:
        ExecuteMsg::BondNymNode {
            node,
            cost_params,
            owner_signature,
        } => crate::nodes::transactions::try_add_nym_node(
            deps,
            env,
            info,
            node,
            cost_params,
            owner_signature,
        ),
        ExecuteMsg::UnbondNymNode {} => {
            crate::nodes::transactions::try_remove_nym_node(deps, env, info)
        }
        ExecuteMsg::UpdateNodeConfig { update } => {
            crate::nodes::transactions::try_update_node_config(deps, info, update)
        }

        // nym-node/mixnode-related:
        ExecuteMsg::PledgeMore {} => {
            crate::compat::transactions::try_increase_pledge(deps, env, info)
        }
        ExecuteMsg::DecreasePledge { decrease_by } => {
            crate::compat::transactions::try_decrease_pledge(deps, env, info, decrease_by)
        }
        ExecuteMsg::UpdateCostParams { new_costs } => {
            crate::compat::transactions::try_update_cost_params(deps, env, info, new_costs)
        }

        // delegation-related:
        ExecuteMsg::Delegate { node_id } => {
            crate::delegations::transactions::try_delegate_to_node(deps, env, info, node_id)
        }
        ExecuteMsg::Undelegate { node_id } => {
            crate::delegations::transactions::try_remove_delegation_from_node(
                deps, env, info, node_id,
            )
        }

        // reward-related
        ExecuteMsg::RewardNode { node_id, params } => {
            crate::rewards::transactions::try_reward_node(deps, env, info, node_id, params)
        }

        ExecuteMsg::WithdrawOperatorReward {} => {
            crate::compat::transactions::try_withdraw_operator_reward(deps, info)
        }
        ExecuteMsg::WithdrawDelegatorReward { node_id: mix_id } => {
            crate::rewards::transactions::try_withdraw_delegator_reward(deps, info, mix_id)
        }

        // vesting migration:
        ExecuteMsg::MigrateVestedMixNode { .. } => {
            crate::vesting_migration::try_migrate_vested_mixnode(deps, info)
        }
        ExecuteMsg::MigrateVestedDelegation { mix_id } => {
            crate::vesting_migration::try_migrate_vested_delegation(deps, info, mix_id)
        }

        // legacy vesting
        ExecuteMsg::BondMixnodeOnBehalf { .. }
        | ExecuteMsg::PledgeMoreOnBehalf { .. }
        | ExecuteMsg::DecreasePledgeOnBehalf { .. }
        | ExecuteMsg::UnbondMixnodeOnBehalf { .. }
        | ExecuteMsg::UpdateMixnodeCostParamsOnBehalf { .. }
        | ExecuteMsg::UpdateMixnodeConfigOnBehalf { .. }
        | ExecuteMsg::BondGatewayOnBehalf { .. }
        | ExecuteMsg::UnbondGatewayOnBehalf { .. }
        | ExecuteMsg::UpdateGatewayConfigOnBehalf { .. }
        | ExecuteMsg::DelegateToMixnodeOnBehalf { .. }
        | ExecuteMsg::UndelegateFromMixnodeOnBehalf { .. }
        | ExecuteMsg::WithdrawOperatorRewardOnBehalf { .. }
        | ExecuteMsg::WithdrawDelegatorRewardOnBehalf { .. } => {
            Err(MixnetContractError::DisabledVestingOperation)
        }

        // testing-only
        #[cfg(feature = "contract-testing")]
        ExecuteMsg::TestingResolveAllPendingEvents { limit } => {
            crate::testing::transactions::try_resolve_all_pending_events(deps, env, limit)
        }
        ExecuteMsg::TestingUncheckedBondLegacyMixnode { node } => {
            legacy::save_new_mixnode(
                deps.storage,
                env,
                node,
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(20).unwrap(),
                    interval_operating_cost: coin(40_000_000, "unym"),
                },
                info.sender,
                info.funds[0].clone(),
            )?;
            Ok(Response::default())
        }
        ExecuteMsg::TestingUncheckedBondLegacyGateway { node } => {
            legacy::save_new_gateway(deps.storage, env, node, info.sender, info.funds[0].clone())?;
            Ok(Response::default())
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
        QueryMsg::Admin {} => to_binary(&crate::mixnet_contract_settings::queries::query_admin(
            deps,
        )?),
        QueryMsg::GetRewardingParams {} => {
            to_binary(&crate::rewards::queries::query_rewarding_params(deps)?)
        }
        QueryMsg::GetEpochStatus {} => {
            to_binary(&crate::interval::queries::query_epoch_status(deps)?)
        }
        QueryMsg::GetCurrentIntervalDetails {} => to_binary(
            &crate::interval::queries::query_current_interval_details(deps, env)?,
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
        QueryMsg::GetPreassignedGatewayIds { limit, start_after } => to_binary(
            &crate::gateways::queries::query_preassigned_ids_paged(deps, start_after, limit)?,
        ),

        // nym-node-related:
        QueryMsg::GetNymNodeBondsPaged { start_after, limit } => to_binary(
            &crate::nodes::queries::query_nymnode_bonds_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetNymNodesDetailedPaged { limit, start_after } => to_binary(
            &crate::nodes::queries::query_nymnodes_details_paged(deps, start_after, limit)?,
        ),
        QueryMsg::GetUnbondedNymNode { node_id } => to_binary(
            &crate::nodes::queries::query_unbonded_nymnode(deps, node_id)?,
        ),
        QueryMsg::GetUnbondedNymNodesPaged { limit, start_after } => to_binary(
            &crate::nodes::queries::query_unbonded_nymnodes_paged(deps, limit, start_after)?,
        ),
        QueryMsg::GetUnbondedNymNodesByOwnerPaged {
            owner,
            limit,
            start_after,
        } => to_binary(
            &crate::nodes::queries::query_unbonded_nymnodes_by_owner_paged(
                deps,
                owner,
                limit,
                start_after,
            )?,
        ),
        QueryMsg::GetUnbondedNymNodesByIdentityKeyPaged {
            identity_key,
            limit,
            start_after,
        } => to_binary(
            &crate::nodes::queries::query_unbonded_nymnodes_by_identity_paged(
                deps,
                identity_key,
                limit,
                start_after,
            )?,
        ),
        QueryMsg::GetOwnedNymNode { address } => {
            to_binary(&crate::nodes::queries::query_owned_nymnode(deps, address)?)
        }
        QueryMsg::GetNymNodeDetails { node_id } => to_binary(
            &crate::nodes::queries::query_nymnode_details(deps, node_id)?,
        ),
        QueryMsg::GetNymNodeDetailsByIdentityKey { node_identity } => to_binary(
            &crate::nodes::queries::query_nymnode_details_by_identity(deps, node_identity)?,
        ),
        QueryMsg::GetNodeRewardingDetails { node_id } => to_binary(
            &crate::nodes::queries::query_nymnode_rewarding_details(deps, node_id)?,
        ),
        QueryMsg::GetNodeStakeSaturation { node_id } => to_binary(
            &crate::nodes::queries::query_stake_saturation(deps, node_id)?,
        ),
        QueryMsg::GetRoleAssignment { role } => {
            to_binary(&crate::nodes::queries::query_epoch_assignment(deps, role)?)
        }
        QueryMsg::GetRewardedSetMetadata {} => {
            to_binary(&crate::nodes::queries::query_rewarded_set_metadata(deps)?)
        }

        // delegation-related:
        QueryMsg::GetNodeDelegations {
            node_id,
            start_after,
            limit,
        } => to_binary(&crate::delegations::queries::query_node_delegations_paged(
            deps,
            node_id,
            start_after,
            limit,
        )?),
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
            node_id,
            delegator,
            proxy,
        } => to_binary(&crate::delegations::queries::query_node_delegation(
            deps, node_id, delegator, proxy,
        )?),
        QueryMsg::GetAllDelegations { start_after, limit } => to_binary(
            &crate::delegations::queries::query_all_delegations_paged(deps, start_after, limit)?,
        ),

        // rewards related
        QueryMsg::GetPendingOperatorReward { address } => to_binary(
            &crate::rewards::queries::query_pending_operator_reward(deps, address)?,
        ),
        QueryMsg::GetPendingNodeOperatorReward { node_id } => to_binary(
            &crate::rewards::queries::query_pending_mixnode_operator_reward(deps, node_id)?,
        ),
        QueryMsg::GetPendingDelegatorReward {
            address,
            node_id,
            proxy,
        } => to_binary(&crate::rewards::queries::query_pending_delegator_reward(
            deps, address, node_id, proxy,
        )?),
        QueryMsg::GetEstimatedCurrentEpochOperatorReward {
            node_id,
            estimated_performance,
            estimated_work,
        } => to_binary(
            &crate::rewards::queries::query_estimated_current_epoch_operator_reward(
                deps,
                node_id,
                estimated_performance,
                estimated_work,
            )?,
        ),
        QueryMsg::GetEstimatedCurrentEpochDelegatorReward {
            address,
            node_id,
            estimated_performance,
            estimated_work,
        } => to_binary(
            &crate::rewards::queries::query_estimated_current_epoch_delegator_reward(
                deps,
                address,
                node_id,
                estimated_performance,
                estimated_work,
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
    mut deps: DepsMut<'_>,
    _env: Env,
    msg: MigrateMsg,
) -> Result<Response, MixnetContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // remove all family-related things
    crate::queued_migrations::families_purge(deps.branch())?;

    // prepare the ground for using nym-nodes rather than standalone mixnodes/gateways
    migrate_to_nym_nodes_usage(deps.branch(), &msg)?;

    // remove all family-related things
    crate::queued_migrations::families_purge(deps.branch())?;

    // prepare the ground for using nym-nodes rather than standalone mixnodes/gateways
    migrate_to_nym_nodes_usage(deps.branch(), &msg)?;

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
    use crate::rewards::storage as rewards_storage;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Decimal, Uint128};
    use mixnet_contract_common::reward_params::{
        IntervalRewardParams, RewardedSetParams, RewardingParams,
    };
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
                rewarded_set_params: RewardedSetParams {
                    entry_gateways: 123,
                    exit_gateways: 70,
                    mixnodes: 120,
                    standby: 0,
                },
            },
            profit_margin: ProfitMarginRange {
                minimum: "0.05".parse().unwrap(),
                maximum: "0.95".parse().unwrap(),
            },
            interval_operating_cost: OperatingCostRange {
                minimum: "1000".parse().unwrap(),
                maximum: "10000".parse().unwrap(),
            },
        };

        let sender = mock_info("sender", &[]);
        let res = instantiate(deps.as_mut(), env, sender, init_msg);
        assert!(res.is_ok());

        #[allow(deprecated)]
        let expected_state = ContractState {
            owner: Some(Addr::unchecked("sender")),
            rewarding_validator_address: Addr::unchecked("foomp123"),
            vesting_contract_address: Addr::unchecked("bar456"),
            rewarding_denom: "uatom".into(),
            params: ContractStateParams {
                minimum_delegation: None,
                minimum_pledge: Coin {
                    denom: "uatom".into(),
                    amount: INITIAL_PLEDGE_AMOUNT,
                },
                profit_margin: ProfitMarginRange {
                    minimum: Percent::from_percentage_value(5).unwrap(),
                    maximum: Percent::from_percentage_value(95).unwrap(),
                },
                interval_operating_cost: OperatingCostRange {
                    minimum: Uint128::new(1000),
                    maximum: Uint128::new(10000),
                },
            },
        };

        let expected_epoch_reward_budget =
            Decimal::from_ratio(100_000_000_000_000u128, 1234u32) * Decimal::percent(1);
        let expected_stake_saturation_point = Decimal::from_ratio(123_456_000_000_000u128, 313u32);

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
            rewarded_set: RewardedSetParams {
                entry_gateways: 123,
                exit_gateways: 70,
                mixnodes: 120,
                standby: 0,
            },
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
