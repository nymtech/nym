// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::push_new_interval_event;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::{must_get_mixnode_bond_by_owner, save_new_mixnode};
use crate::support::helpers::{
    ensure_bonded, ensure_no_existing_bond, ensure_proxy_match, validate_node_identity_signature,
    validate_pledge,
};
use cosmwasm_std::{Addr, Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_mixnode_bonding_event, new_mixnode_config_update_event,
    new_mixnode_pending_cost_params_update_event, new_pending_mixnode_unbonding_event,
};
use mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use mixnet_contract_common::MixNode;

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
    ensure_no_existing_bond(deps.storage, &owner)?;

    // there's no need to explicitly check whether there already exists mixnode with the same
    // identity or sphinx keys as this is going to be done implicitly when attempting to save
    // the bond information due to `UniqueIndex` constraint defined on those fields.

    // check if this sender actually owns the mixnode by checking the signature
    validate_node_identity_signature(
        deps.as_ref(),
        &owner,
        owner_signature,
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

pub fn try_remove_mixnode_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_remove_mixnode(deps, owner, Some(proxy))
}

pub fn try_remove_mixnode(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_remove_mixnode(deps, info.sender, None)
}

pub(crate) fn _try_remove_mixnode(
    deps: DepsMut<'_>,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    let existing_bond = storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond { owner })?
        .1;

    // see if the proxy matches
    ensure_proxy_match(&proxy, &existing_bond.proxy)?;
    ensure_bonded(&existing_bond)?;

    // set `is_unbonding` field
    let mut updated_bond = existing_bond.clone();
    updated_bond.is_unbonding = true;
    storage::mixnode_bonds().replace(
        deps.storage,
        existing_bond.id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    // push the event to execute it at the end of the epoch
    let epoch_event = PendingEpochEvent::UnbondMixnode {
        mix_id: existing_bond.id,
    };
    interval_storage::push_new_epoch_event(deps.storage, &epoch_event)?;

    Ok(
        Response::new().add_event(new_pending_mixnode_unbonding_event(
            &existing_bond.owner,
            &existing_bond.proxy,
            existing_bond.identity(),
            existing_bond.id,
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
        new_mixnode_config_update_event(existing_bond.id, &owner, &proxy, &new_config);

    let mut updated_bond = existing_bond.clone();
    updated_bond.mix_node.host = new_config.host;
    updated_bond.mix_node.mix_port = new_config.mix_port;
    updated_bond.mix_node.verloc_port = new_config.verloc_port;
    updated_bond.mix_node.http_api_port = new_config.http_api_port;
    updated_bond.mix_node.version = new_config.version;

    storage::mixnode_bonds().replace(
        deps.storage,
        existing_bond.id,
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    Ok(Response::new().add_event(cfg_update_event))
}

pub(crate) fn try_update_mixnode_cost_params(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_costs: MixNodeCostParams,
) -> Result<Response, MixnetContractError> {
    let owner = info.sender;
    _try_update_mixnode_cost_params(deps, new_costs, owner, None)
}

pub(crate) fn try_update_mixnode_cost_params_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    new_costs: MixNodeCostParams,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let owner = deps.api.addr_validate(&owner)?;
    let proxy = info.sender;
    _try_update_mixnode_cost_params(deps, new_costs, owner, Some(proxy))
}

pub(crate) fn _try_update_mixnode_cost_params(
    deps: DepsMut,
    new_costs: MixNodeCostParams,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // see if the node still exists
    let existing_bond = must_get_mixnode_bond_by_owner(deps.storage, &owner)?;

    ensure_proxy_match(&proxy, &existing_bond.proxy)?;
    ensure_bonded(&existing_bond)?;

    let cosmos_event =
        new_mixnode_pending_cost_params_update_event(existing_bond.id, &owner, &proxy, &new_costs);

    // push the interval event
    let interval_event = PendingIntervalEvent::ChangeMixCostParams {
        mix: existing_bond.id,
        new_costs,
    };
    push_new_interval_event(deps.storage, &interval_event)?;

    Ok(Response::new().add_event(cosmos_event))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::mixnet_contract_settings::storage::minimum_mixnode_pledge;
    use crate::mixnodes::helpers::get_mixnode_details_by_id;
    use crate::support::tests::fixtures::{good_mixnode_pledge, TEST_COIN_DENOM};
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Order, StdResult, Uint128};
    use mixnet_contract_common::{ExecuteMsg, Layer, LayerDistribution, Percent};

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
        let (mixnode, sig) = test_helpers::mixnode_with_signature(&mut rng, sender);
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
        let (mixnode, sig) = test_helpers::mixnode_with_signature(&mut rng, sender2);

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
        assert_eq!(2, bond.id);

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
    fn mixnode_remove() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let sender = "alice";
        let info = mock_info(sender, &[]);

        // trying to remove your mixnode fails if you never had one in the first place
        let res = try_remove_mixnode(deps.as_mut(), info.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(sender)
            })
        );

        let mix_id =
            test_helpers::add_mixnode(&mut rng, deps.as_mut(), env, sender, good_mixnode_pledge());

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_remove_mixnode_on_behalf(
            deps.as_mut(),
            mock_info("proxy", &[]),
            sender.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: "proxy".to_string()
            })
        );

        // "normal" unbonding succeeds and unbonding event is pushed to the pending epoch events
        let res = try_remove_mixnode(deps.as_mut(), info.clone());
        assert!(res.is_ok());
        let mut pending_events = interval_storage::PENDING_EPOCH_EVENTS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(pending_events.len(), 1);
        let event = pending_events.pop().unwrap();
        assert_eq!(1, event.0);
        assert_eq!(PendingEpochEvent::UnbondMixnode { mix_id }, event.1);

        // but fails if repeated (since the node is already in the "unbonding" state)(
        let res = try_remove_mixnode(deps.as_mut(), info);
        assert_eq!(
            res,
            Err(MixnetContractError::MixnodeIsUnbonding { node_id: mix_id })
        )
    }

    #[test]
    fn updating_mixnode_config() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let sender = "alice";
        let info = mock_info(sender, &[]);
        let update = MixNodeConfigUpdate {
            host: "1.1.1.1:1234".to_string(),
            mix_port: 1234,
            verloc_port: 1235,
            http_api_port: 1236,
            version: "v1.2.3".to_string(),
        };

        // try updating a non existing mixnode bond
        let res = try_update_mixnode_config(deps.as_mut(), info.clone(), update.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(sender)
            })
        );

        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env,
            sender,
            tests::fixtures::good_mixnode_pledge(),
        );

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_update_mixnode_config_on_behalf(
            deps.as_mut(),
            mock_info("proxy", &[]),
            update.clone(),
            sender.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: "proxy".to_string()
            })
        );
        // "normal" update succeeds
        let res = try_update_mixnode_config(deps.as_mut(), info.clone(), update.clone());
        assert!(res.is_ok());

        // and the config has actually been updated
        let mix = must_get_mixnode_bond_by_owner(deps.as_ref().storage, &Addr::unchecked(sender))
            .unwrap();
        assert_eq!(mix.mix_node.host, update.host);
        assert_eq!(mix.mix_node.mix_port, update.mix_port);
        assert_eq!(mix.mix_node.verloc_port, update.verloc_port);
        assert_eq!(mix.mix_node.http_api_port, update.http_api_port);
        assert_eq!(mix.mix_node.version, update.version);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(deps.as_mut(), info.clone()).unwrap();
        let res = try_update_mixnode_config(deps.as_mut(), info, update);
        assert_eq!(
            res,
            Err(MixnetContractError::MixnodeIsUnbonding { node_id: mix_id })
        )
    }

    #[test]
    fn updating_mixnode_cost_params() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        let sender = "alice";
        let info = mock_info(sender, &[]);
        let update = MixNodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
            interval_operating_cost: Coin::new(12345678, TEST_COIN_DENOM),
        };

        // try updating a non existing mixnode bond
        let res = try_update_mixnode_cost_params(deps.as_mut(), info.clone(), update.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked(sender)
            })
        );

        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            sender,
            tests::fixtures::good_mixnode_pledge(),
        );

        // attempted to remove on behalf with invalid proxy (current is `None`)
        let res = try_update_mixnode_cost_params_on_behalf(
            deps.as_mut(),
            mock_info("proxy", &[]),
            update.clone(),
            sender.to_string(),
        );
        assert_eq!(
            res,
            Err(MixnetContractError::ProxyMismatch {
                existing: "None".to_string(),
                incoming: "proxy".to_string()
            })
        );
        // "normal" update succeeds
        let res = try_update_mixnode_cost_params(deps.as_mut(), info.clone(), update.clone());
        assert!(res.is_ok());

        // see if the event has been pushed onto the queue
        let mut pending_events = interval_storage::PENDING_INTERVAL_EVENTS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(pending_events.len(), 1);
        let event = pending_events.pop().unwrap();
        assert_eq!(1, event.0);
        assert_eq!(
            PendingIntervalEvent::ChangeMixCostParams {
                mix: mix_id,
                new_costs: update.clone()
            },
            event.1
        );

        // execute the event
        test_helpers::execute_all_pending_events(deps.as_mut(), env);

        // and see if the config has actually been updated
        let mix = get_mixnode_details_by_id(deps.as_ref().storage, mix_id)
            .unwrap()
            .unwrap();
        assert_eq!(mix.rewarding_details.cost_params, update);

        // but we cannot perform any updates whilst the mixnode is already unbonding
        try_remove_mixnode(deps.as_mut(), info.clone()).unwrap();
        let res = try_update_mixnode_cost_params(deps.as_mut(), info, update);
        assert_eq!(
            res,
            Err(MixnetContractError::MixnodeIsUnbonding { node_id: mix_id })
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
}
