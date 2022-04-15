// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{ACTIVE_SET_WORK_FACTOR, INTERVAL_REWARD_PERCENT, SYBIL_RESISTANCE_PERCENT};
use crate::delegations::queries::query_delegator_delegations_paged;
use crate::delegations::queries::query_mixnode_delegation;
use crate::delegations::queries::{
    query_mixnode_delegations_paged, query_pending_delegation_events,
};
use crate::error::ContractError;
use crate::gateways::queries::query_gateways_paged;
use crate::gateways::queries::query_owns_gateway;
use crate::interval::queries::query_current_epoch;
use crate::interval::queries::{
    query_current_rewarded_set_height, query_rewarded_set,
    query_rewarded_set_refresh_minimum_blocks, query_rewarded_set_update_details,
};
use crate::interval::transactions::{init_epoch, try_init_epoch};
use crate::mixnet_contract_settings::models::ContractState;
use crate::mixnet_contract_settings::queries::{
    query_contract_settings_params, query_contract_version, query_rewarding_validator_address,
};
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnet_contract_settings::transactions::try_update_rewarding_validator_address;
use crate::mixnodes::bonding_queries as mixnode_queries;
use crate::mixnodes::bonding_queries::query_mixnodes_paged;
use crate::mixnodes::layer_queries::query_layer_distribution;
use crate::rewards::queries::{
    query_circulating_supply, query_reward_pool, query_rewarding_status,
};
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, Uint128,
};
use mixnet_contract_common::{
    ContractStateParams, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use time::OffsetDateTime;

/// Constant specifying minimum of coin required to bond a gateway
pub const INITIAL_GATEWAY_PLEDGE: Uint128 = Uint128::new(100_000_000);

/// Constant specifying minimum of coin required to bond a mixnode
pub const INITIAL_MIXNODE_PLEDGE: Uint128 = Uint128::new(100_000_000);

pub const INITIAL_MIXNODE_REWARDED_SET_SIZE: u32 = 200;
pub const INITIAL_MIXNODE_ACTIVE_SET_SIZE: u32 = 100;

pub const INITIAL_REWARD_POOL: u128 = 250_000_000_000_000;
pub const INITIAL_ACTIVE_SET_WORK_FACTOR: u8 = 10;

pub const DEFAULT_FIRST_INTERVAL_START: OffsetDateTime =
    time::macros::datetime!(2022-01-01 12:00 UTC);

fn default_initial_state(owner: Addr, rewarding_validator_address: Addr) -> ContractState {
    ContractState {
        owner,
        rewarding_validator_address,
        params: ContractStateParams {
            minimum_mixnode_pledge: INITIAL_MIXNODE_PLEDGE,
            minimum_gateway_pledge: INITIAL_GATEWAY_PLEDGE,
            mixnode_rewarded_set_size: INITIAL_MIXNODE_REWARDED_SET_SIZE,
            mixnode_active_set_size: INITIAL_MIXNODE_ACTIVE_SET_SIZE,
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
) -> Result<Response, ContractError> {
    let rewarding_validator_address = deps.api.addr_validate(&msg.rewarding_validator_address)?;
    let state = default_initial_state(info.sender, rewarding_validator_address);
    init_epoch(deps.storage, env)?;

    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;
    mixnet_params_storage::LAYERS.save(deps.storage, &Default::default())?;
    rewards_storage::REWARD_POOL.save(deps.storage, &Uint128::new(INITIAL_REWARD_POOL))?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateRewardingValidatorAddress { address } => {
            try_update_rewarding_validator_address(deps, info, address)
        }
        ExecuteMsg::InitEpoch {} => try_init_epoch(info, deps.storage, env),
        ExecuteMsg::BondMixnode {
            mix_node,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode(
            deps,
            env,
            info,
            mix_node,
            owner_signature,
        ),
        ExecuteMsg::UnbondMixnode {} => {
            crate::mixnodes::transactions::try_remove_mixnode(env, deps, info)
        }
        ExecuteMsg::UpdateMixnodeConfig {
            profit_margin_percent,
        } => crate::mixnodes::transactions::try_update_mixnode_config(
            deps,
            env,
            info,
            profit_margin_percent,
        ),
        ExecuteMsg::UpdateMixnodeConfigOnBehalf {
            profit_margin_percent,
            owner,
        } => crate::mixnodes::transactions::try_update_mixnode_config_on_behalf(
            deps,
            env,
            info,
            profit_margin_percent,
            owner,
        ),
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
        ExecuteMsg::UpdateContractStateParams(params) => {
            crate::mixnet_contract_settings::transactions::try_update_contract_settings(
                deps, info, params,
            )
        }
        ExecuteMsg::RewardMixnode { identity, params } => {
            crate::rewards::transactions::try_reward_mixnode(deps, env, info, identity, params)
        }
        ExecuteMsg::DelegateToMixnode { mix_identity } => {
            crate::delegations::transactions::try_delegate_to_mixnode(deps, env, info, mix_identity)
        }
        ExecuteMsg::UndelegateFromMixnode { mix_identity } => {
            crate::delegations::transactions::try_remove_delegation_from_mixnode(
                deps,
                env,
                info,
                mix_identity,
            )
        }
        // ExecuteMsg::RewardNextMixDelegators {
        //     mix_identity,
        //     interval_id,
        // } => crate::rewards::transactions::try_reward_next_mixnode_delegators(
        //     deps,
        //     info,
        //     mix_identity,
        //     interval_id,
        // ),
        ExecuteMsg::DelegateToMixnodeOnBehalf {
            mix_identity,
            delegate,
        } => crate::delegations::transactions::try_delegate_to_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_identity,
            delegate,
        ),
        ExecuteMsg::UndelegateFromMixnodeOnBehalf {
            mix_identity,
            delegate,
        } => crate::delegations::transactions::try_remove_delegation_from_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_identity,
            delegate,
        ),
        ExecuteMsg::BondMixnodeOnBehalf {
            mix_node,
            owner,
            owner_signature,
        } => crate::mixnodes::transactions::try_add_mixnode_on_behalf(
            deps,
            env,
            info,
            mix_node,
            owner,
            owner_signature,
        ),
        ExecuteMsg::UnbondMixnodeOnBehalf { owner } => {
            crate::mixnodes::transactions::try_remove_mixnode_on_behalf(env, deps, info, owner)
        }
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
        ExecuteMsg::UnbondGatewayOnBehalf { owner } => {
            crate::gateways::transactions::try_remove_gateway_on_behalf(deps, info, owner)
        }
        ExecuteMsg::WriteRewardedSet {
            rewarded_set,
            expected_active_set_size,
        } => crate::interval::transactions::try_write_rewarded_set(
            deps,
            env,
            info,
            rewarded_set,
            expected_active_set_size,
        ),
        ExecuteMsg::AdvanceCurrentEpoch {} => crate::interval::transactions::try_advance_epoch(
            env,
            deps.storage,
            info.sender.to_string(),
        ),
        ExecuteMsg::CompoundDelegatorReward { mix_identity } => {
            crate::rewards::transactions::try_compound_delegator_reward(
                deps,
                env,
                info,
                mix_identity,
            )
        }
        ExecuteMsg::CompoundOperatorReward {} => {
            crate::rewards::transactions::try_compound_operator_reward(deps, env, info)
        }
        ExecuteMsg::CompoundDelegatorRewardOnBehalf {
            owner,
            mix_identity,
        } => crate::rewards::transactions::try_compound_delegator_reward_on_behalf(
            deps,
            env,
            info,
            owner,
            mix_identity,
        ),
        ExecuteMsg::CompoundOperatorRewardOnBehalf { owner } => {
            crate::rewards::transactions::try_compound_operator_reward_on_behalf(
                deps, env, info, owner,
            )
        }
        ExecuteMsg::ReconcileDelegations {} => {
            crate::delegations::transactions::try_reconcile_all_delegation_events(deps, info)
        }
        ExecuteMsg::CheckpointMixnodes {} => {
            crate::mixnodes::transactions::try_checkpoint_mixnodes(
                deps.storage,
                env.block.height,
                info,
            )
        }
    }
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetRewardingValidatorAddress {} => {
            to_binary(&query_rewarding_validator_address(deps)?)
        }
        QueryMsg::GetContractVersion {} => to_binary(&query_contract_version()),
        QueryMsg::GetMixNodes { start_after, limit } => {
            to_binary(&query_mixnodes_paged(deps, start_after, limit)?)
        }
        QueryMsg::GetGateways { limit, start_after } => {
            to_binary(&query_gateways_paged(deps, start_after, limit)?)
        }
        QueryMsg::OwnsMixnode { address } => {
            to_binary(&mixnode_queries::query_owns_mixnode(deps, address)?)
        }
        QueryMsg::OwnsGateway { address } => to_binary(&query_owns_gateway(deps, address)?),
        QueryMsg::StateParams {} => to_binary(&query_contract_settings_params(deps)?),
        QueryMsg::LayerDistribution {} => to_binary(&query_layer_distribution(deps)?),
        QueryMsg::GetMixnodeDelegations {
            mix_identity,
            start_after,
            limit,
        } => to_binary(&query_mixnode_delegations_paged(
            deps,
            mix_identity,
            start_after,
            limit,
        )?),
        QueryMsg::GetDelegatorDelegations {
            delegator: delegation_owner,
            start_after,
            limit,
        } => to_binary(&query_delegator_delegations_paged(
            deps,
            delegation_owner,
            start_after,
            limit,
        )?),
        QueryMsg::GetDelegationDetails {
            mix_identity,
            delegator,
            proxy,
        } => to_binary(&query_mixnode_delegation(
            deps.storage,
            deps.api,
            mix_identity,
            delegator,
            proxy,
        )?),
        QueryMsg::GetRewardPool {} => to_binary(&query_reward_pool(deps)?),
        QueryMsg::GetCirculatingSupply {} => to_binary(&query_circulating_supply(deps)?),
        QueryMsg::GetIntervalRewardPercent {} => to_binary(&INTERVAL_REWARD_PERCENT),
        QueryMsg::GetSybilResistancePercent {} => to_binary(&SYBIL_RESISTANCE_PERCENT),
        QueryMsg::GetActiveSetWorkFactor {} => to_binary(&ACTIVE_SET_WORK_FACTOR),
        QueryMsg::GetRewardingStatus {
            mix_identity,
            interval_id,
        } => to_binary(&query_rewarding_status(deps, mix_identity, interval_id)?),
        QueryMsg::GetRewardedSet {
            height,
            start_after,
            limit,
        } => to_binary(&query_rewarded_set(
            deps.storage,
            height,
            start_after,
            limit,
        )?),
        QueryMsg::GetRewardedSetUpdateDetails {} => {
            to_binary(&query_rewarded_set_update_details(env, deps.storage)?)
        }
        QueryMsg::GetCurrentRewardedSetHeight {} => {
            to_binary(&query_current_rewarded_set_height(deps.storage)?)
        }
        // QueryMsg::GetCurrentInterval {} => to_binary(&query_current_interval(deps.storage)?),
        QueryMsg::GetRewardedSetRefreshBlocks {} => {
            to_binary(&query_rewarded_set_refresh_minimum_blocks())
        }
        QueryMsg::GetEpochsInInterval {} => to_binary(&crate::constants::EPOCHS_IN_INTERVAL),
        QueryMsg::GetCurrentEpoch {} => to_binary(&query_current_epoch(deps.storage)?),
        QueryMsg::QueryOperatorReward { address } => to_binary(
            &crate::rewards::queries::query_operator_reward(deps, address)?,
        ),
        QueryMsg::QueryDelegatorReward {
            address,
            mix_identity,
        } => to_binary(&crate::rewards::queries::query_delegator_reward(
            deps,
            address,
            mix_identity,
        )?),
        QueryMsg::GetPendingDelegationEvents { owner_address } => to_binary(
            &query_pending_delegation_events(deps.storage, owner_address)?,
        ),
        QueryMsg::GetAllDelegationKeys {} => to_binary(
            &crate::delegations::queries::query_all_delegation_keys(deps.storage)?,
        ),
        QueryMsg::DebugGetAllDelegationValues {} => to_binary(
            &crate::delegations::queries::debug_query_all_delegation_values(deps.storage)?,
        ),
    };

    Ok(query_res?)
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::support::tests;
    use config::defaults::{DEFAULT_NETWORK, DENOM};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use mixnet_contract_common::PagedMixnodeResponse;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {
            rewarding_validator_address: DEFAULT_NETWORK.rewarding_validator_address().to_string(),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mix_node_bonds should be empty after initialization
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedMixnodeResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len()); // there are no mixnodes in the list when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, DENOM),
            tests::queries::query_contract_balance(env.contract.address, deps)
        );
    }

    #[cfg(test)]
    mod migration {
        use super::*;
        use crate::mixnodes::storage::{mixnodes, StoredMixnodeBond};
        use crate::support::tests::test_helpers;
        use cw_storage_plus::{Index, IndexList, IndexedSnapshotMap, Strategy, UniqueIndex};
        use mixnet_contract_common::{Addr, IdentityKeyRef, Layer, MixNode};
        use rand::thread_rng;

        const MIXNODES_PK_NAMESPACE: &str = "mn";
        const MIXNODES_PK_CHECKPOINTS: &str = "mn__check";
        const MIXNODES_PK_CHANGELOG: &str = "mn__change";
        const MIXNODES_OWNER_IDX_NAMESPACE: &str = "mno";

        pub(crate) struct MixnodeBondIndex<'a> {
            pub(crate) owner: UniqueIndex<'a, Addr, StoredMixnodeBond>,
        }

        // IndexList is just boilerplate code for fetching a struct's indexes
        // note that from my understanding this will be converted into a macro at some point in the future
        impl<'a> IndexList<StoredMixnodeBond> for MixnodeBondIndex<'a> {
            fn get_indexes(
                &'_ self,
            ) -> Box<dyn Iterator<Item = &'_ dyn Index<StoredMixnodeBond>> + '_> {
                let v: Vec<&dyn Index<StoredMixnodeBond>> = vec![&self.owner];
                Box::new(v.into_iter())
            }
        }

        pub(crate) fn old_mixnodes<'a>(
        ) -> IndexedSnapshotMap<'a, IdentityKeyRef<'a>, StoredMixnodeBond, MixnodeBondIndex<'a>>
        {
            let indexes = MixnodeBondIndex {
                owner: UniqueIndex::new(|d| d.owner.clone(), MIXNODES_OWNER_IDX_NAMESPACE),
            };
            IndexedSnapshotMap::new(
                MIXNODES_PK_NAMESPACE,
                MIXNODES_PK_CHECKPOINTS,
                MIXNODES_PK_CHANGELOG,
                Strategy::Never,
                indexes,
            )
        }

        fn change_stored_bond(store: &mut dyn Storage, key: &str, height: u64) {
            // just to have something changing, update ports to some stupid values
            let mut old = old_mixnodes().load(store, key).unwrap();
            old.mix_node.mix_port += 1;
            old.mix_node.verloc_port -= 1;

            let reward = old.accumulated_rewards.unwrap_or_default();
            old.accumulated_rewards = Some(reward + Uint128::new(42));

            old_mixnodes().save(store, key, &old, height).unwrap()
        }

        fn bond_old_mixnode(
            store: &mut dyn Storage,
            sender: &str,
            height: u64,
        ) -> (String, String) {
            let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
            let legit_sphix = crypto::asymmetric::encryption::KeyPair::new(&mut thread_rng());
            let key = keypair.public_key().to_base58_string();
            let key2 = legit_sphix.public_key().to_base58_string();

            old_mixnodes()
                .save(
                    store,
                    &key,
                    &StoredMixnodeBond {
                        pledge_amount: Default::default(),
                        owner: Addr::unchecked(sender),
                        layer: Layer::One,
                        block_height: height,
                        mix_node: MixNode {
                            host: "1.2.3.4".to_string(),
                            mix_port: 1234,
                            verloc_port: 1234,
                            http_api_port: 123,
                            sphinx_key: key2.clone(),
                            identity_key: key.clone(),
                            version: "1.2.3".to_string(),
                            profit_margin_percent: 42,
                        },
                        proxy: None,
                        accumulated_rewards: None,
                        epoch_rewards: None,
                    },
                    height,
                )
                .unwrap();

            (key, key2)
        }

        #[test]
        fn empty() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();
            assert!(migrate(deps.as_mut(), env, MigrateMsg {}).is_ok())
        }

        fn check_mixnode(storage: &dyn Storage, owner: Addr, id: &str, expected_sphinx: &str) {
            assert_eq!(
                mixnodes().load(storage, &id).unwrap().mix_node.sphinx_key,
                expected_sphinx
            );
            assert_eq!(
                mixnodes()
                    .idx
                    .owner
                    .item(storage, owner)
                    .unwrap()
                    .unwrap()
                    .1
                    .mix_node
                    .sphinx_key,
                expected_sphinx
            );
            assert_eq!(
                mixnodes()
                    .idx
                    .sphinx_key
                    .item(storage, expected_sphinx.to_owned())
                    .unwrap()
                    .unwrap()
                    .1
                    .mix_node
                    .sphinx_key,
                expected_sphinx
            );
        }

        #[test]
        fn single_bond() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();

            let (id, sphinx) = bond_old_mixnode(deps.as_mut().storage, "bob", 42);

            assert!(migrate(deps.as_mut(), env, MigrateMsg {}).is_ok());
            check_mixnode(deps.as_ref().storage, Addr::unchecked("bob"), &id, &sphinx);
        }

        #[test]
        fn multiple_bonds() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();

            let (id1, sphinx1) = bond_old_mixnode(deps.as_mut().storage, "bob", 42);
            let (id2, sphinx2) = bond_old_mixnode(deps.as_mut().storage, "alice", 43);
            let (id3, sphinx3) = bond_old_mixnode(deps.as_mut().storage, "eve", 44);

            assert!(migrate(deps.as_mut(), env, MigrateMsg {}).is_ok());
            check_mixnode(
                deps.as_ref().storage,
                Addr::unchecked("bob"),
                &id1,
                &sphinx1,
            );
            check_mixnode(
                deps.as_ref().storage,
                Addr::unchecked("alice"),
                &id2,
                &sphinx2,
            );
            check_mixnode(
                deps.as_ref().storage,
                Addr::unchecked("eve"),
                &id3,
                &sphinx3,
            );
        }

        #[test]
        fn multiple_bonds_with_history() {
            let mut deps = test_helpers::init_contract();
            let env = mock_env();

            let (id1, sphinx1) = bond_old_mixnode(deps.as_mut().storage, "bob", 42);
            let (id2, sphinx2) = bond_old_mixnode(deps.as_mut().storage, "alice", 43);
            let (id3, sphinx3) = bond_old_mixnode(deps.as_mut().storage, "eve", 44);

            // make alice change her bond multiple times
            change_stored_bond(deps.as_mut().storage, &id2, 1234);
            change_stored_bond(deps.as_mut().storage, &id2, 4326);
            change_stored_bond(deps.as_mut().storage, &id2, 432576);
            change_stored_bond(deps.as_mut().storage, &id2, 5474746);

            assert!(migrate(deps.as_mut(), env, MigrateMsg {}).is_ok());
            check_mixnode(
                deps.as_ref().storage,
                Addr::unchecked("bob"),
                &id1,
                &sphinx1,
            );
            check_mixnode(
                deps.as_ref().storage,
                Addr::unchecked("alice"),
                &id2,
                &sphinx2,
            );
            check_mixnode(
                deps.as_ref().storage,
                Addr::unchecked("eve"),
                &id3,
                &sphinx3,
            );
        }
    }
}
