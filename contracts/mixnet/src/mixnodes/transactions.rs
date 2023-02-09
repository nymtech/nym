// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::push_new_interval_event;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnet_contract_settings::storage::rewarding_denom;
use crate::mixnodes::helpers::{
    get_mixnode_details_by_owner, must_get_mixnode_bond_by_owner, save_new_mixnode,
};
use crate::support::helpers::{
    ensure_bonded, ensure_epoch_in_progress_state, ensure_is_authorized, ensure_no_existing_bond,
    ensure_proxy_match, ensure_sent_by_vesting_contract, validate_node_identity_signature,
    validate_pledge,
};
use cosmwasm_std::{coin, Addr, Coin, DepsMut, Env, MessageInfo, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_mixnode_bonding_event, new_mixnode_config_update_event,
    new_mixnode_pending_cost_params_update_event, new_pending_mixnode_unbonding_event,
    new_pending_pledge_increase_event,
};
use mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use mixnet_contract_common::pending_events::{PendingEpochEventKind, PendingIntervalEventKind};
use mixnet_contract_common::{Layer, MixId, MixNode};

pub(crate) fn update_mixnode_layer(
    mix_id: MixId,
    layer: Layer,
    storage: &mut dyn Storage,
) -> Result<(), MixnetContractError> {
    let bond = if let Some(bond_information) = storage::mixnode_bonds().may_load(storage, mix_id)? {
        bond_information
    } else {
        return Err(MixnetContractError::MixNodeBondNotFound { mix_id });
    };
    let mut updated_bond = bond.clone();
    updated_bond.layer = layer;

    storage::mixnode_bonds().replace(storage, bond.mix_id, Some(&updated_bond), Some(&bond))?;
    Ok(())
}

pub fn assign_mixnode_layer(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: MixId,
    layer: Layer,
) -> Result<Response, MixnetContractError> {
    ensure_is_authorized(&info.sender, deps.storage)?;

    update_mixnode_layer(mix_id, layer, deps.storage)?;

    Ok(Response::default())
}

pub fn try_add_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
    owner_signature: String,
) -> Result<Response, MixnetContractError> {
    _try_add_mixnode(
        deps,
        env,
        mix_node,
        cost_params,
        info.funds,
        info.sender,
        owner_signature,
        None,
    )
}

pub fn try_add_mixnode_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
    owner: String,
    owner_signature: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_add_mixnode(
        deps,
        env,
        mix_node,
        cost_params,
        info.funds,
        owner,
        owner_signature,
        Some(proxy),
    )
}

// I'm not entirely sure how to deal with this warning at the current moment
#[allow(clippy::too_many_arguments)]
fn _try_add_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    pledge: Vec<Coin>,
    owner: Addr,
    owner_signature: String,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::minimum_mixnode_pledge(deps.storage)?;
    let pledge = validate_pledge(pledge, minimum_pledge)?;

    // if the client has an active bonded mixnode or gateway, don't allow bonding
    // note that this has to be done explicitly as `UniqueIndex` constraint would not protect us
    // against attempting to use different node types (i.e. gateways and mixnodes)
    ensure_no_existing_bond(&owner, deps.storage)?;

    // there's no need to explicitly check whether there already exists mixnode with the same
    // identity or sphinx keys as this is going to be done implicitly when attempting to save
    // the bond information due to `UniqueIndex` constraint defined on those fields.

    // check if this sender actually owns the mixnode by checking the signature
    validate_node_identity_signature(
        deps.as_ref(),
        &owner,
        &owner_signature,
        &mixnode.identity_key,
    )?;

    let node_identity = mixnode.identity_key.clone();
    let (node_id, layer) = save_new_mixnode(
        deps.storage,
        env,
        mixnode,
        cost_params,
        owner.clone(),
        proxy.clone(),
        pledge.clone(),
    )?;

    Ok(Response::new().add_event(new_mixnode_bonding_event(
        &owner,
        &proxy,
        &pledge,
        &node_identity,
        node_id,
        layer,
    )))
}

pub fn try_increase_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_increase_pledge(deps, env, info.funds, info.sender, None)
}

pub fn try_increase_pledge_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_increase_pledge(deps, env, info.funds, owner, Some(proxy))
}

pub fn _try_increase_pledge(
    deps: DepsMut<'_>,
    env: Env,
    increase: Vec<Coin>,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let mix_details = get_mixnode_details_by_owner(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond { owner })?;
    let mix_id = mix_details.mix_id();

    // increasing pledge is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    ensure_proxy_match(&proxy, &mix_details.bond_information.proxy)?;
    ensure_bonded(&mix_details.bond_information)?;

    let rewarding_denom = rewarding_denom(deps.storage)?;
    let pledge_increase = validate_pledge(increase, coin(1, rewarding_denom))?;

    let cosmos_event = new_pending_pledge_increase_event(mix_id, &pledge_increase);

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::PledgeMore {
        mix_id,
        amount: pledge_increase,
    };
    interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub fn try_remove_mixnode_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_remove_mixnode(deps, env, owner, Some(proxy))
}

pub fn try_remove_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_remove_mixnode(deps, env, info.sender, None)
}

pub(crate) fn _try_remove_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond = storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond { owner })?
        .1;

    // unbonding is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    // see if the proxy matches
    ensure_proxy_match(&proxy, &existing_bond.proxy)?;
    ensure_bonded(&existing_bond)?;

    // set `is_unbonding` field
    let mut updated_bond = existing_bond.clone();
    updated_bond.is_unbonding = true;
    storage::mixnode_bonds().replace(
        deps.storage,
        existing_bond.mix_id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEventKind::UnbondMixnode {
        mix_id: existing_bond.mix_id,
    };
    interval_storage::push_new_epoch_event(deps.storage, &env, epoch_event)?;

    Ok(
        Response::new().add_event(new_pending_mixnode_unbonding_event(
            &existing_bond.owner,
            &existing_bond.proxy,
            existing_bond.identity(),
            existing_bond.mix_id,
        )),
    )
}

pub(crate) fn try_update_mixnode_config(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_config: MixNodeConfigUpdate,
) -> Result<Response, MixnetContractError> {
    let owner = info.sender;
    _try_update_mixnode_config(deps, new_config, owner, None)
}

pub(crate) fn try_update_mixnode_config_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    new_config: MixNodeConfigUpdate,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let owner = deps.api.addr_validate(&owner)?;
    let proxy = info.sender;
    _try_update_mixnode_config(deps, new_config, owner, Some(proxy))
}

pub(crate) fn _try_update_mixnode_config(
    deps: DepsMut,
    new_config: MixNodeConfigUpdate,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    ensure_bonded(&existing_bond)?;
    ensure_proxy_match(&proxy, &existing_bond.proxy)?;

    let cfg_update_event =
        new_mixnode_config_update_event(existing_bond.mix_id, &owner, &proxy, &new_config);

    let mut updated_bond = existing_bond.clone();
    updated_bond.mix_node.host = new_config.host;
    updated_bond.mix_node.mix_port = new_config.mix_port;
    updated_bond.mix_node.verloc_port = new_config.verloc_port;
    updated_bond.mix_node.http_api_port = new_config.http_api_port;
    updated_bond.mix_node.version = new_config.version;

    storage::mixnode_bonds().replace(
        deps.storage,
        existing_bond.mix_id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    Ok(Response::new().add_event(cfg_update_event))
}

pub(crate) fn try_update_mixnode_cost_params(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    new_costs: MixNodeCostParams,
) -> Result<Response, MixnetContractError> {
    let owner = info.sender;
    _try_update_mixnode_cost_params(deps, env, new_costs, owner, None)
}

pub(crate) fn try_update_mixnode_cost_params_on_behalf(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_costs: MixNodeCostParams,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let owner = deps.api.addr_validate(&owner)?;
    let proxy = info.sender;
    _try_update_mixnode_cost_params(deps, env, new_costs, owner, Some(proxy))
}

pub(crate) fn _try_update_mixnode_cost_params(
    deps: DepsMut,
    env: Env,
    new_costs: MixNodeCostParams,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // see if the node still exists
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    // changing cost params is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(deps.storage)?;

    ensure_proxy_match(&proxy, &existing_bond.proxy)?;
    ensure_bonded(&existing_bond)?;

    let cosmos_event = new_mixnode_pending_cost_params_update_event(
        existing_bond.mix_id,
        &owner,
        &proxy,
        &new_costs,
    );

    // push the interval event
    let interval_event = PendingIntervalEventKind::ChangeMixCostParams {
        mix_id: existing_bond.mix_id,
        new_costs,
    };
    push_new_interval_event(deps.storage, &env, interval_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::mixnet_contract_settings::storage::minimum_mixnode_pledge;
    use crate::mixnodes::helpers::get_mixnode_details_by_id;
    use crate::support::tests::fixtures::{good_mixnode_pledge, TEST_COIN_DENOM};
    use crate::support::tests::test_helpers::TestSetup;
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Order, StdResult, Uint128};
    use mixnet_contract_common::{
        EpochState, EpochStatus, ExecuteMsg, Layer, LayerDistribution, Percent,
    };

    #[test]
    fn mixnode_add() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let sender = "alice";
        let minimum_pledge = minimum_mixnode_pledge(deps.as_ref().storage).unwrap();
        let mut insufficient_pledge = minimum_pledge.clone();
        insufficient_pledge.amount -= Uint128::new(1000);

        // if we don't send enough funds
        let info = mock_info(sender, &[insufficient_pledge.clone()]);
        let (mixnode, sig, _) = test_helpers::mixnode_with_signature(&mut rng, sender);
        let cost_params = fixtures::mix_node_cost_params_fixture();

        // we are informed that we didn't send enough funds
        let result = try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            mixnode.clone(),
            cost_params.clone(),
            sig.clone(),
        );
        assert_eq!(
            result,
            Err(MixnetContractError::InsufficientPledge {
                received: insufficient_pledge,
                minimum: minimum_pledge.clone(),
            })
        );

        // if the signature provided is invalid, the bonding also fails
        let info = mock_info(sender, &[minimum_pledge]);

        let result = try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            mixnode.clone(),
            cost_params.clone(),
            "bad-signature".into(),
        );
        assert!(matches!(
            result,
            Err(MixnetContractError::MalformedEd25519Signature(..))
        ));

        // if there was already a mixnode bonded by particular user
        test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            sender,
            fixtures::good_mixnode_pledge(),
        );

        // it fails
        let result = try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            mixnode,
            cost_params.clone(),
            sig,
        );
        assert_eq!(Err(MixnetContractError::AlreadyOwnsMixnode), result);

        // the same holds if the user already owns a gateway
        let sender2 = "gateway-owner";

        test_helpers::add_gateway(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            sender2,
            tests::fixtures::good_gateway_pledge(),
        );

        let info = mock_info(sender2, &tests::fixtures::good_mixnode_pledge());
        let (mixnode, sig, _) = test_helpers::mixnode_with_signature(&mut rng, sender2);

        let result = try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            mixnode.clone(),
            cost_params.clone(),
            sig.clone(),
        );
        assert_eq!(Err(MixnetContractError::AlreadyOwnsGateway), result);

        // but after he unbonds it, it's all fine again
        let msg = ExecuteMsg::UnbondGateway {};
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let result = try_add_mixnode(deps.as_mut(), env, info, mixnode, cost_params, sig);
        assert!(result.is_ok());

        // make sure we got assigned the next id (note: we have already bonded a mixnode before in this test)
        let bond = must_get_mixnode_bond_by_owner(deps.as_ref().storage, &Addr::unchecked(sender2))
            .unwrap();
        assert_eq!(2, bond.mix_id);

        // and make sure we're on layer 2 (because it was the next empty one)
        assert_eq!(Layer::Two, bond.layer);

        // and see if the layer distribution matches our expectation
        let expected = LayerDistribution {
            layer1: 1,
            layer2: 1,
            layer3: 0,
        };
        assert_eq!(
            expected,
            storage::LAYERS.load(deps.as_ref().storage).unwrap()
        )
    }

    #[test]
    fn mixnode_add_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";
        let (mixnode, sig, _) = test_helpers::mixnode_with_signature(&mut test.rng, owner);
        let cost_params = fixtures::mix_node_cost_params_fixture();

        // we are informed that we didn't send enough funds
        let res = try_add_mixnode_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &good_mixnode_pledge()),
            mixnode,
            cost_params,
            owner.to_string(),
            sig,
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract
            }
        )
    }

    #[test]
    fn removing_mixnode_cant_be_performed_if_epoch_transition_is_in_progress() {
        let bad_states = vec![
            EpochState::Rewarding {
                last_rewarded: 0,
                final_node_id: 0,
            },
            EpochState::ReconcilingEvents,
            EpochState::AdvancingEpoch,
        ];

        for bad_state in bad_states {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "alice";
            let info = mock_info(owner, &[]);

            test.add_dummy_mixnode(owner, None);

            let mut status = EpochStatus::new(test.rewarding_validator().sender);
            status.state = bad_state;
            interval_storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

            let res = try_remove_mixnode(test.deps_mut(), env.clone(), info);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochAdvancementInProgress { .. })
            ));
        }
    }

    #[test]
    fn mixnode_remove() {
        let mut test = TestSetup::new();
        let env = test.env();

        let owner = "alice";
        let info = mock_info(owner, &[]);

        // trying to remove your mixnode fails if you never had one in the first place
        let res = try_remove_mixnode(test.deps_mut(), env.clone(), info.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(owner)
            })
        );

        let mix_id = test.add_dummy_mixnode(owner, None);
        let vesting_contract = test.vesting_contract();

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_remove_mixnode_on_behalf(
            test.deps_mut(),
            env.clone(),
            mock_info(vesting_contract.as_ref(), &[]),
            owner.to_string(),
        );

        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: vesting_contract.into_string()
            })
        );

        // "normal" unbonding succeeds and unbonding event is pushed to the pending epoch events
        let res = try_remove_mixnode(test.deps_mut(), env.clone(), info.clone());
        assert!(res.is_ok());
        let mut pending_events = interval_storage::PENDING_EPOCH_EVENTS
            .range(test.deps().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(pending_events.len(), 1);
        let event = pending_events.pop().unwrap();
        assert_eq!(1, event.0);
        assert_eq!(
            PendingEpochEventKind::UnbondMixnode { mix_id },
            event.1.kind
        );

        // but fails if repeated (since the node is already in the "unbonding" state)(
        let res = try_remove_mixnode(test.deps_mut(), env, info);
        assert_eq!(res, Err(MixnetContractError::MixnodeIsUnbonding { mix_id }))
    }

    #[test]
    fn mixnode_remove_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());

        let res = try_remove_mixnode_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &[]),
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract
            }
        )
    }

    #[test]
    fn updating_mixnode_config() {
        let mut test = TestSetup::new();
        let env = test.env();

        let owner = "alice";
        let info = mock_info(owner, &[]);
        let update = MixNodeConfigUpdate {
            host: "1.1.1.1:1234".to_string(),
            mix_port: 1234,
            verloc_port: 1235,
            http_api_port: 1236,
            version: "v1.2.3".to_string(),
        };

        // try updating a non existing mixnode bond
        let res = try_update_mixnode_config(test.deps_mut(), info.clone(), update.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(owner)
            })
        );

        let mix_id = test.add_dummy_mixnode(owner, None);
        let vesting_contract = test.vesting_contract();

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_update_mixnode_config_on_behalf(
            test.deps_mut(),
            mock_info(vesting_contract.as_ref(), &[]),
            update.clone(),
            owner.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: vesting_contract.into_string()
            })
        );
        // "normal" update succeeds
        let res = try_update_mixnode_config(test.deps_mut(), info.clone(), update.clone());
        assert!(res.is_ok());

        // and the config has actually been updated
        let mix =
            must_get_mixnode_bond_by_owner(test.deps().storage, &Addr::unchecked(owner)).unwrap();
        assert_eq!(mix.mix_node.host, update.host);
        assert_eq!(mix.mix_node.mix_port, update.mix_port);
        assert_eq!(mix.mix_node.verloc_port, update.verloc_port);
        assert_eq!(mix.mix_node.http_api_port, update.http_api_port);
        assert_eq!(mix.mix_node.version, update.version);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(test.deps_mut(), env, info.clone()).unwrap();
        let res = try_update_mixnode_config(test.deps_mut(), info, update);
        assert_eq!(res, Err(MixnetContractError::MixnodeIsUnbonding { mix_id }))
    }

    #[test]
    fn updating_mixnode_config_with_illegal_proxy() {
        let mut test = TestSetup::new();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());
        let update = MixNodeConfigUpdate {
            host: "1.1.1.1:1234".to_string(),
            mix_port: 1234,
            verloc_port: 1235,
            http_api_port: 1236,
            version: "v1.2.3".to_string(),
        };

        let res = try_update_mixnode_config_on_behalf(
            test.deps_mut(),
            mock_info(illegal_proxy.as_ref(), &[]),
            update,
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract
            }
        )
    }

    #[test]
    fn mixnode_cost_params_cant_be_updated_when_epoch_transition_is_in_progress() {
        let bad_states = vec![
            EpochState::Rewarding {
                last_rewarded: 0,
                final_node_id: 0,
            },
            EpochState::ReconcilingEvents,
            EpochState::AdvancingEpoch,
        ];

        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        for bad_state in bad_states {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "alice";
            let info = mock_info(owner, &[]);

            test.add_dummy_mixnode(owner, None);

            let mut status = EpochStatus::new(test.rewarding_validator().sender);
            status.state = bad_state;
            interval_storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

            let res =
                try_update_mixnode_cost_params(test.deps_mut(), env.clone(), info, update.clone());
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochAdvancementInProgress { .. })
            ));
        }
    }

    #[test]
    fn updating_mixnode_cost_params() {
        let mut test = TestSetup::new();
        let env = test.env();

        let owner = "alice";
        let info = mock_info(owner, &[]);
        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        // try updating a non existing mixnode bond
        let res = try_update_mixnode_cost_params(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            update.clone(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(owner)
            })
        );

        let mix_id = test.add_dummy_mixnode(owner, None);
        let vesting_contract = test.vesting_contract();

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_update_mixnode_cost_params_on_behalf(
            test.deps_mut(),
            env.clone(),
            mock_info(vesting_contract.as_ref(), &[]),
            update.clone(),
            owner.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: vesting_contract.into_string()
            })
        );
        // "normal" update succeeds
        let res = try_update_mixnode_cost_params(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            update.clone(),
        );
        assert!(res.is_ok());

        // see if the event has been pushed onto the queue
        let mut pending_events = interval_storage::PENDING_INTERVAL_EVENTS
            .range(test.deps().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(pending_events.len(), 1);
        let event = pending_events.pop().unwrap();
        assert_eq!(1, event.0);
        assert_eq!(
            PendingIntervalEventKind::ChangeMixCostParams {
                mix_id,
                new_costs: update.clone()
            },
            event.1.kind
        );

        // execute the event
        test_helpers::execute_all_pending_events(test.deps_mut(), env.clone());

        // and see if the config has actually been updated
        let mix = get_mixnode_details_by_id(test.deps().storage, mix_id)
            .unwrap()
            .unwrap();
        assert_eq!(mix.rewarding_details.cost_params, update);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(test.deps_mut(), env.clone(), info.clone()).unwrap();
        let res = try_update_mixnode_cost_params(test.deps_mut(), env, info, update);
        assert_eq!(res, Err(MixnetContractError::MixnodeIsUnbonding { mix_id }))
    }

    #[test]
    fn updating_mixnode_cost_params_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());
        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        let res = try_update_mixnode_cost_params_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &[]),
            update,
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract
            }
        )
    }

    #[test]
    fn adding_mixnode_with_duplicate_sphinx_key_errors_out() {
        let mut deps = test_helpers::init_contract();
        let mut rng = test_helpers::test_rng();

        let keypair1 = crypto::asymmetric::identity::KeyPair::new(&mut rng);
        let keypair2 = crypto::asymmetric::identity::KeyPair::new(&mut rng);
        let sig1 = keypair1.private_key().sign_text("alice");
        let sig2 = keypair1.private_key().sign_text("bob");

        let info_alice = mock_info("alice", &tests::fixtures::good_mixnode_pledge());
        let info_bob = mock_info("bob", &tests::fixtures::good_mixnode_pledge());

        let mut mixnode = MixNode {
            host: "1.2.3.4".to_string(),
            mix_port: 1234,
            verloc_port: 1234,
            http_api_port: 1234,
            sphinx_key: crypto::asymmetric::encryption::KeyPair::new(&mut rng)
                .public_key()
                .to_base58_string(),
            identity_key: keypair1.public_key().to_base58_string(),
            version: "v0.1.2.3".to_string(),
        };
        let cost_params = fixtures::mix_node_cost_params_fixture();

        assert!(try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            info_alice,
            mixnode.clone(),
            cost_params.clone(),
            sig1
        )
        .is_ok());

        mixnode.identity_key = keypair2.public_key().to_base58_string();

        // change identity but reuse sphinx key
        assert!(try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            info_bob,
            mixnode,
            cost_params,
            sig2
        )
        .is_err());
    }

    #[cfg(test)]
    mod increasing_mixnode_pledge {
        use super::*;
        use crate::mixnodes::helpers::tests::{
            setup_mix_combinations, OWNER_UNBONDED, OWNER_UNBONDED_LEFTOVER, OWNER_UNBONDING,
        };
        use crate::support::tests::test_helpers::TestSetup;
        use mixnet_contract_common::{EpochState, EpochStatus};

        #[test]
        fn cant_be_performed_if_epoch_transition_is_in_progress() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let env = test.env();
                let owner = "mix-owner";

                test.add_dummy_mixnode(owner, None);

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let sender = mock_info(owner, &[test.coin(1000)]);
                let res = try_increase_pledge(test.deps_mut(), env, sender);

                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));
            }
        }

        #[test]
        fn is_not_allowed_if_account_doesnt_own_mixnode() {
            let mut test = TestSetup::new();
            let env = test.env();
            let sender = mock_info("not-mix-owner", &[]);

            let res = try_increase_pledge(test.deps_mut(), env, sender);
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: Addr::unchecked("not-mix-owner")
                })
            )
        }

        #[test]
        fn is_not_allowed_if_theres_proxy_mismatch() {
            let mut test = TestSetup::new();
            let env = test.env();

            let owner_without_proxy = Addr::unchecked("no-proxy");
            let owner_with_proxy = Addr::unchecked("with-proxy");
            let proxy = Addr::unchecked("proxy");
            let wrong_proxy = Addr::unchecked("unrelated-proxy");

            test.add_dummy_mixnode(owner_without_proxy.as_str(), None);
            test.add_dummy_mixnode_with_illegal_proxy(
                owner_with_proxy.as_str(),
                None,
                proxy.clone(),
            );

            let res = _try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                Vec::new(),
                owner_without_proxy.clone(),
                Some(proxy),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "None".to_string(),
                    incoming: "proxy".to_string()
                })
            );

            let res = _try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                Vec::new(),
                owner_with_proxy.clone(),
                None,
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "proxy".to_string(),
                    incoming: "None".to_string()
                })
            );

            let res = _try_increase_pledge(
                test.deps_mut(),
                env,
                Vec::new(),
                owner_with_proxy.clone(),
                Some(wrong_proxy),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::ProxyMismatch {
                    existing: "proxy".to_string(),
                    incoming: "unrelated-proxy".to_string()
                })
            )
        }

        #[test]
        fn is_not_allowed_if_mixnode_has_unbonded_or_is_unbonding() {
            let mut test = TestSetup::new();
            let env = test.env();

            // TODO: I dislike this cross-test access, but it provides us with exactly what we need
            // perhaps it should be refactored a bit?
            let owner_unbonding = Addr::unchecked(OWNER_UNBONDING);
            let owner_unbonded = Addr::unchecked(OWNER_UNBONDED);
            let owner_unbonded_leftover = Addr::unchecked(OWNER_UNBONDED_LEFTOVER);

            let ids = setup_mix_combinations(&mut test);
            let mix_id_unbonding = ids[1];

            let res = try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                mock_info(owner_unbonding.as_str(), &[]),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: mix_id_unbonding
                })
            );

            // if the nodes are gone we treat them as tey never existed in the first place
            // (regardless of if there's some leftover data)
            let res = try_increase_pledge(
                test.deps_mut(),
                env.clone(),
                mock_info(owner_unbonded_leftover.as_str(), &[]),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: owner_unbonded_leftover
                })
            );

            let res = try_increase_pledge(
                test.deps_mut(),
                env,
                mock_info(owner_unbonded.as_str(), &[]),
            );
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: owner_unbonded
                })
            )
        }

        #[test]
        fn is_not_allowed_if_no_tokens_were_sent() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";

            test.add_dummy_mixnode(owner, None);

            let sender_empty = mock_info(owner, &[]);
            let res = try_increase_pledge(test.deps_mut(), env.clone(), sender_empty);
            assert_eq!(res, Err(MixnetContractError::NoBondFound));

            let sender_zero = mock_info(owner, &[test.coin(0)]);
            let res = try_increase_pledge(test.deps_mut(), env, sender_zero);
            assert_eq!(
                res,
                Err(MixnetContractError::InsufficientPledge {
                    received: test.coin(0),
                    minimum: test.coin(1)
                })
            )
        }

        #[test]
        fn with_valid_information_creates_pending_event() {
            let mut test = TestSetup::new();
            let env = test.env();
            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, None);

            let events = test.pending_epoch_events();
            assert!(events.is_empty());

            let sender = mock_info(owner, &[test.coin(1000)]);
            try_increase_pledge(test.deps_mut(), env, sender).unwrap();

            let events = test.pending_epoch_events();

            assert_eq!(
                events[0].kind,
                PendingEpochEventKind::PledgeMore {
                    mix_id,
                    amount: test.coin(1000)
                }
            );
        }
    }

    #[test]
    fn fails_for_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_mixnode_with_illegal_proxy(owner, None, illegal_proxy.clone());

        let res = try_increase_pledge_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &[coin(123, TEST_COIN_DENOM)]),
            owner.to_string(),
        )
        .unwrap_err();

        assert_eq!(
            res,
            MixnetContractError::SenderIsNotVestingContract {
                received: illegal_proxy,
                vesting_contract
            }
        )
    }
}
