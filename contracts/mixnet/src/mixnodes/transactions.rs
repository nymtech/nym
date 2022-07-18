// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::push_new_interval_event;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::save_new_mixnode;
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
    let existing_bond = storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        })?
        .1;

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
    let existing_bond = storage::mixnode_bonds()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
        .ok_or(MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        })?
        .1;

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

// #[cfg(test)]
// pub mod tests {
//     use super::*;
//     use crate::contract::{execute, query, INITIAL_MIXNODE_PLEDGE};
//     use crate::error::ContractError;
//     use crate::mixnodes::transactions::validate_mixnode_pledge;
//     use crate::support::tests;
//     use crate::support::tests::test_helpers;
//     use config::defaults::MIX_DENOM;
//     use cosmwasm_std::testing::{mock_env, mock_info};
//     use cosmwasm_std::{coins, BankMsg, Response};
//     use cosmwasm_std::{from_binary, Addr, Uint128};
//     use mixnet_contract_common::{
//         ExecuteMsg, Layer, LayerDistribution, MixNode, PagedMixnodeResponse, QueryMsg,
//     };
//     use rand::thread_rng;
//
//     #[test]
//     fn mixnode_add() {
//         let mut deps = test_helpers::init_contract();
//
//         // if we don't send enough funds
//         let insufficient_bond = Into::<u128>::into(INITIAL_MIXNODE_PLEDGE) - 1;
//         let info = mock_info("anyone", &coins(insufficient_bond, MIX_DENOM.base));
//         let (msg, _) = tests::messages::valid_bond_mixnode_msg("anyone");
//
//         // we are informed that we didn't send enough funds
//         let result = execute(deps.as_mut(), mock_env(), info, msg);
//         assert_eq!(
//             result,
//             Err(ContractError::InsufficientMixNodeBond {
//                 received: insufficient_bond,
//                 minimum: INITIAL_MIXNODE_PLEDGE.into(),
//             })
//         );
//
//         // no mixnode was inserted into the topology
//         let res = query(
//             deps.as_ref(),
//             mock_env(),
//             QueryMsg::GetMixNodes {
//                 start_after: None,
//                 limit: Option::from(2),
//             },
//         )
//         .unwrap();
//         let page: PagedMixnodeResponse = from_binary(&res).unwrap();
//         assert_eq!(0, page.nodes.len());
//
//         // if we send enough funds
//         let info = mock_info("anyone", &tests::fixtures::good_mixnode_pledge());
//         let (msg, (identity, sphinx)) = tests::messages::valid_bond_mixnode_msg("anyone");
//
//         // we get back a message telling us everything was OK
//         let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
//         assert!(execute_response.is_ok());
//
//         // we can query topology and the new node is there
//         let query_response = query(
//             deps.as_ref(),
//             mock_env(),
//             QueryMsg::GetMixNodes {
//                 start_after: None,
//                 limit: Option::from(2),
//             },
//         )
//         .unwrap();
//         let page: PagedMixnodeResponse = from_binary(&query_response).unwrap();
//         assert_eq!(1, page.nodes.len());
//         assert_eq!(
//             &MixNode {
//                 identity_key: identity,
//                 sphinx_key: sphinx,
//                 ..tests::fixtures::mix_node_fixture()
//             },
//             page.nodes[0].mix_node()
//         );
//
//         // if there was already a mixnode bonded by particular user
//         let info = mock_info("foomper", &tests::fixtures::good_mixnode_pledge());
//         let (msg, _) = tests::messages::valid_bond_mixnode_msg("foomper");
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//
//         let info = mock_info("foomper", &tests::fixtures::good_mixnode_pledge());
//         let (msg, _) = tests::messages::valid_bond_mixnode_msg("foomper");
//
//         // it fails
//         let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
//         assert_eq!(Err(ContractError::AlreadyOwnsMixnode), execute_response);
//
//         // bonding fails if the user already owns a gateway
//         test_helpers::add_gateway(
//             "gateway-owner",
//             tests::fixtures::good_gateway_pledge(),
//             deps.as_mut(),
//         );
//
//         let info = mock_info("gateway-owner", &tests::fixtures::good_mixnode_pledge());
//         let (msg, _) = tests::messages::valid_bond_mixnode_msg("gateway-owner");
//
//         let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
//         assert_eq!(execute_response, Err(ContractError::AlreadyOwnsGateway));
//
//         // but after he unbonds it, it's all fine again
//         let info = mock_info("gateway-owner", &[]);
//         let msg = ExecuteMsg::UnbondGateway {};
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//
//         let info = mock_info("gateway-owner", &tests::fixtures::good_mixnode_pledge());
//         let (msg, _) = tests::messages::valid_bond_mixnode_msg("gateway-owner");
//
//         let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
//         assert!(execute_response.is_ok());
//
//         // adding another node from another account, but with the same IP, should fail (or we would have a weird state). Is that right? Think about this, not sure yet.
//         // if we attempt to register a second node from the same address, should we get an error? It would probably be polite.
//     }
//
//     #[test]
//     fn adding_mixnode_without_existing_owner_succeeds() {
//         let mut deps = test_helpers::init_contract();
//
//         let info = mock_info("mix-owner", &tests::fixtures::good_mixnode_pledge());
//
//         // before the execution the node had no associated owner
//         assert!(storage::mixnodes()
//             .idx
//             .owner
//             .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
//             .unwrap()
//             .is_none());
//
//         let (msg, (identity, _)) = tests::messages::valid_bond_mixnode_msg("mix-owner");
//
//         // it's all fine, owner is saved
//         let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
//         assert!(execute_response.is_ok());
//
//         assert_eq!(
//             &identity,
//             storage::mixnodes()
//                 .idx
//                 .owner
//                 .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
//                 .unwrap()
//                 .unwrap()
//                 .1
//                 .identity()
//         );
//     }
//
//     #[test]
//     fn adding_mixnode_with_existing_owner_fails() {
//         let mut deps = test_helpers::init_contract();
//
//         let identity = test_helpers::add_mixnode(
//             "mix-owner",
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         // request fails giving the existing owner address in the message
//         let info = mock_info(
//             "mix-owner-pretender",
//             &tests::fixtures::good_mixnode_pledge(),
//         );
//         let msg = ExecuteMsg::BondMixnode {
//             mix_node: MixNode {
//                 identity_key: identity,
//                 ..tests::fixtures::mix_node_fixture()
//             },
//             owner_signature: "foomp".to_string(),
//         };
//
//         let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
//         assert_eq!(
//             Err(ContractError::DuplicateMixnode {
//                 owner: Addr::unchecked("mix-owner")
//             }),
//             execute_response
//         );
//     }
//
//     #[test]
//     fn adding_mixnode_with_existing_unchanged_owner_fails() {
//         let mut deps = test_helpers::init_contract();
//
//         test_helpers::add_mixnode(
//             "mix-owner",
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         let info = mock_info("mix-owner", &tests::fixtures::good_mixnode_pledge());
//         let (msg, _) = tests::messages::valid_bond_mixnode_msg("mix-owner");
//
//         let res = execute(deps.as_mut(), mock_env(), info, msg);
//         assert_eq!(Err(ContractError::AlreadyOwnsMixnode), res);
//     }
//
//     #[test]
//     fn adding_mixnode_updates_layer_distribution() {
//         let mut deps = test_helpers::init_contract();
//
//         assert_eq!(
//             LayerDistribution::default(),
//             mixnet_params_storage::LAYERS.load(&deps.storage).unwrap(),
//         );
//
//         test_helpers::add_mixnode(
//             "mix1",
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         assert_eq!(
//             LayerDistribution {
//                 layer1: 1,
//                 ..Default::default()
//             },
//             mixnet_params_storage::LAYERS.load(&deps.storage).unwrap()
//         );
//     }
//
//     #[test]
//     fn mixnode_remove() {
//         let mut deps = test_helpers::init_contract();
//
//         // try un-registering when no nodes exist yet
//         let info = mock_info("anyone", &[]);
//         let msg = ExecuteMsg::UnbondMixnode {};
//         let result = execute(deps.as_mut(), mock_env(), info, msg);
//
//         // we're told that there is no node for our address
//         assert_eq!(
//             result,
//             Err(ContractError::NoAssociatedMixNodeBond {
//                 owner: Addr::unchecked("anyone")
//             })
//         );
//
//         // let's add a node owned by bob
//         test_helpers::add_mixnode("bob", tests::fixtures::good_mixnode_pledge(), deps.as_mut());
//
//         // attempt to un-register fred's node, which doesn't exist
//         let info = mock_info("fred", &[]);
//         let msg = ExecuteMsg::UnbondMixnode {};
//         let result = execute(deps.as_mut(), mock_env(), info, msg);
//         assert_eq!(
//             result,
//             Err(ContractError::NoAssociatedMixNodeBond {
//                 owner: Addr::unchecked("fred")
//             })
//         );
//
//         // bob's node is still there
//         let nodes = tests::queries::get_mix_nodes(&mut deps);
//         assert_eq!(1, nodes.len());
//         assert_eq!("bob", nodes[0].owner().clone());
//
//         // add a node owned by fred
//         let fred_identity = test_helpers::add_mixnode(
//             "fred",
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         // let's make sure we now have 2 nodes:
//         assert_eq!(2, tests::queries::get_mix_nodes(&mut deps).len());
//
//         // un-register fred's node
//         let info = mock_info("fred", &[]);
//         let msg = ExecuteMsg::UnbondMixnode {};
//         let remove_fred = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
//
//         // we should see a funds transfer from the contract back to fred
//         let expected_message = BankMsg::Send {
//             to_address: String::from(info.sender),
//             amount: tests::fixtures::good_mixnode_pledge(),
//         };
//
//         // run the executor and check that we got back the correct results
//         let expected_response =
//             Response::new()
//                 .add_message(expected_message)
//                 .add_event(new_mixnode_unbonding_event(
//                     &Addr::unchecked("fred"),
//                     &None,
//                     &tests::fixtures::good_mixnode_pledge()[0],
//                     &fred_identity,
//                 ));
//
//         assert_eq!(expected_response, remove_fred);
//
//         // only 1 node now exists, owned by bob:
//         let mix_node_bonds = tests::queries::get_mix_nodes(&mut deps);
//
//         assert_eq!(1, mix_node_bonds.len());
//         assert_eq!(&Addr::unchecked("bob"), mix_node_bonds[0].owner());
//     }
//
//     #[test]
//     fn removing_mixnode_clears_ownership() {
//         let mut deps = test_helpers::init_contract();
//
//         let info = mock_info("mix-owner", &tests::fixtures::good_mixnode_pledge());
//         let (bond_msg, (identity, _)) = tests::messages::valid_bond_mixnode_msg("mix-owner");
//         execute(deps.as_mut(), mock_env(), info, bond_msg.clone()).unwrap();
//
//         assert_eq!(
//             &identity,
//             storage::mixnodes()
//                 .idx
//                 .owner
//                 .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
//                 .unwrap()
//                 .unwrap()
//                 .1
//                 .identity()
//         );
//
//         let info = mock_info("mix-owner", &[]);
//         let msg = ExecuteMsg::UnbondMixnode {};
//
//         let response = execute(deps.as_mut(), mock_env(), info, msg);
//
//         assert!(response.is_ok());
//
//         assert!(storage::mixnodes()
//             .idx
//             .owner
//             .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
//             .unwrap()
//             .is_none());
//
//         // and since it's removed, it can be reclaimed
//         let info = mock_info("mix-owner", &tests::fixtures::good_mixnode_pledge());
//
//         assert!(execute(deps.as_mut(), mock_env(), info, bond_msg).is_ok());
//         assert_eq!(
//             &identity,
//             storage::mixnodes()
//                 .idx
//                 .owner
//                 .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
//                 .unwrap()
//                 .unwrap()
//                 .1
//                 .identity()
//         );
//     }
//
//     #[test]
//     fn updating_mixnode_config() {
//         let sender = "bob";
//         let mut env = mock_env();
//         let mut deps = test_helpers::init_contract();
//         let info = mock_info(sender, &[]);
//
//         // try updating a non existing mixnode bond
//         let msg = ExecuteMsg::UpdateMixnodeConfig {
//             profit_margin_percent: 10,
//         };
//         let ret = execute(deps.as_mut(), env.clone(), info.clone(), msg);
//         assert_eq!(
//             ret,
//             Err(ContractError::NoAssociatedMixNodeBond {
//                 owner: Addr::unchecked(sender)
//             })
//         );
//
//         test_helpers::add_mixnode(
//             sender,
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         // check the initial profit margin is set to the fixture value
//         let fixture_profit_margin = tests::fixtures::mix_node_fixture().profit_margin_percent;
//         assert_eq!(
//             fixture_profit_margin,
//             storage::mixnodes()
//                 .idx
//                 .owner
//                 .item(deps.as_ref().storage, Addr::unchecked("bob"))
//                 .unwrap()
//                 .unwrap()
//                 .1
//                 .mix_node
//                 .profit_margin_percent
//         );
//
//         env.block.time = env.block.time.plus_seconds(MIN_PM_UPDATE_INTERVAL + 1);
//
//         // try updating with an invalid value
//         let profit_margin_percent = 101;
//         let msg = ExecuteMsg::UpdateMixnodeConfig {
//             profit_margin_percent,
//         };
//         let ret = execute(deps.as_mut(), env.clone(), info.clone(), msg);
//         assert_eq!(
//             ret,
//             Err(ContractError::InvalidProfitMarginPercent(
//                 profit_margin_percent
//             ))
//         );
//
//         let profit_margin_percent = fixture_profit_margin + 10;
//         let msg = ExecuteMsg::UpdateMixnodeConfig {
//             profit_margin_percent,
//         };
//         execute(deps.as_mut(), env, info, msg).unwrap();
//         assert_eq!(
//             profit_margin_percent,
//             storage::mixnodes()
//                 .idx
//                 .owner
//                 .item(deps.as_ref().storage, Addr::unchecked("bob"))
//                 .unwrap()
//                 .unwrap()
//                 .1
//                 .mix_node
//                 .profit_margin_percent
//         );
//     }
//
//     #[test]
//     fn validating_mixnode_bond() {
//         // you must send SOME funds
//         let result = validate_mixnode_pledge(Vec::new(), INITIAL_MIXNODE_PLEDGE);
//         assert_eq!(result, Err(ContractError::NoBondFound));
//
//         // you must send at least 100 coins...
//         let mut bond = tests::fixtures::good_mixnode_pledge();
//         bond[0].amount = INITIAL_MIXNODE_PLEDGE.checked_sub(Uint128::new(1)).unwrap();
//         let result = validate_mixnode_pledge(bond.clone(), INITIAL_MIXNODE_PLEDGE);
//         assert_eq!(
//             result,
//             Err(ContractError::InsufficientMixNodeBond {
//                 received: Into::<u128>::into(INITIAL_MIXNODE_PLEDGE) - 1,
//                 minimum: INITIAL_MIXNODE_PLEDGE.into(),
//             })
//         );
//
//         // more than that is still fine
//         let mut bond = tests::fixtures::good_mixnode_pledge();
//         bond[0].amount = INITIAL_MIXNODE_PLEDGE + Uint128::new(1);
//         let result = validate_mixnode_pledge(bond.clone(), INITIAL_MIXNODE_PLEDGE);
//         assert!(result.is_ok());
//
//         // it must be sent in the defined denom!
//         let mut bond = tests::fixtures::good_mixnode_pledge();
//         bond[0].denom = "baddenom".to_string();
//         let result = validate_mixnode_pledge(bond.clone(), INITIAL_MIXNODE_PLEDGE);
//         assert_eq!(result, Err(ContractError::WrongDenom {}));
//
//         let mut bond = tests::fixtures::good_mixnode_pledge();
//         bond[0].denom = "foomp".to_string();
//         let result = validate_mixnode_pledge(bond.clone(), INITIAL_MIXNODE_PLEDGE);
//         assert_eq!(result, Err(ContractError::WrongDenom {}));
//     }
//
//     #[test]
//     fn choose_layer_mix_node() {
//         let mut deps = test_helpers::init_contract();
//         let alice_identity = test_helpers::add_mixnode(
//             "alice",
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//         let bob_identity =
//             test_helpers::add_mixnode("bob", tests::fixtures::good_mixnode_pledge(), deps.as_mut());
//
//         let bonded_mix_nodes = tests::queries::get_mix_nodes(&mut deps);
//         let alice_node = bonded_mix_nodes
//             .iter()
//             .find(|m| m.owner == "alice")
//             .cloned()
//             .unwrap();
//         let bob_node = bonded_mix_nodes
//             .iter()
//             .find(|m| m.owner == "bob")
//             .cloned()
//             .unwrap();
//
//         assert_eq!(alice_node.mix_node.identity_key, alice_identity);
//         assert_eq!(alice_node.layer, Layer::One);
//         assert_eq!(bob_node.mix_node.identity_key, bob_identity);
//         assert_eq!(bob_node.layer, mixnet_contract_common::Layer::Two);
//     }
//
//     #[test]
//     fn adding_mixnode_with_duplicate_sphinx_key_errors_out() {
//         let mut deps = test_helpers::init_contract();
//
//         let keypair1 = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
//         let keypair2 = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
//         let sig1 = keypair1.private_key().sign_text("alice");
//         let sig2 = keypair1.private_key().sign_text("bob");
//
//         let info_alice = mock_info("alice", &tests::fixtures::good_mixnode_pledge());
//         let info_bob = mock_info("bob", &tests::fixtures::good_mixnode_pledge());
//
//         let mut mixnode = MixNode {
//             host: "1.2.3.4".to_string(),
//             mix_port: 1234,
//             verloc_port: 1234,
//             http_api_port: 1234,
//             sphinx_key: crypto::asymmetric::encryption::KeyPair::new(&mut thread_rng())
//                 .public_key()
//                 .to_base58_string(),
//             identity_key: keypair1.public_key().to_base58_string(),
//             version: "v0.1.2.3".to_string(),
//             profit_margin_percent: 10,
//         };
//
//         assert!(
//             try_add_mixnode(deps.as_mut(), mock_env(), info_alice, mixnode.clone(), sig1).is_ok()
//         );
//
//         mixnode.identity_key = keypair2.public_key().to_base58_string();
//
//         // change identity but reuse sphinx key
//         assert!(try_add_mixnode(deps.as_mut(), mock_env(), info_bob, mixnode, sig2).is_err());
//     }
//
//     #[test]
//     fn updating_pm_too_often_fails() {
//         use super::MIN_PM_UPDATE_INTERVAL;
//
//         let mut deps = test_helpers::init_contract();
//         let mut env = mock_env();
//
//         let keypair1 = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
//         let sig1 = keypair1.private_key().sign_text("alice");
//
//         let info_alice = mock_info("alice", &tests::fixtures::good_mixnode_pledge());
//
//         let mixnode = MixNode {
//             host: "1.2.3.4".to_string(),
//             mix_port: 1234,
//             verloc_port: 1234,
//             http_api_port: 1234,
//             sphinx_key: crypto::asymmetric::encryption::KeyPair::new(&mut thread_rng())
//                 .public_key()
//                 .to_base58_string(),
//             identity_key: keypair1.public_key().to_base58_string(),
//             version: "v0.1.2.3".to_string(),
//             profit_margin_percent: 10,
//         };
//
//         assert!(try_add_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             info_alice.clone(),
//             mixnode.clone(),
//             sig1
//         )
//         .is_ok());
//
//         env.block.time = env.block.time.plus_seconds(MIN_PM_UPDATE_INTERVAL - 1);
//
//         // fails if too soon after bonding
//         assert!(
//             try_update_mixnode_config(deps.as_mut(), env.clone(), info_alice.clone(), 20).is_err()
//         );
//
//         env.block.time = env.block.time.plus_seconds(2);
//
//         // succeds after some time
//         assert!(try_update_mixnode_config(deps.as_mut(), env, info_alice, 20).is_ok());
//     }
// }
