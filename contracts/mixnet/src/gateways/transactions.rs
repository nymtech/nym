// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::support::helpers::{ensure_no_existing_bond, validate_node_identity_signature};
use config::defaults::DENOM;
use cosmwasm_std::{
    coins, wasm_execute, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128,
};
use mixnet_contract_common::events::{new_gateway_bonding_event, new_gateway_unbonding_event};
use mixnet_contract_common::{Gateway, GatewayBond, Layer};
use vesting_contract::messages::ExecuteMsg as VestingContractExecuteMsg;

pub fn try_add_gateway(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
    owner_signature: String,
) -> Result<Response, ContractError> {
    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::CONTRACT_STATE
        .load(deps.storage)?
        .params
        .minimum_mixnode_pledge;
    let pledge = validate_gateway_pledge(info.funds, minimum_pledge)?;

    _try_add_gateway(
        deps,
        env,
        gateway,
        pledge,
        info.sender.as_str(),
        owner_signature,
        None,
    )
}

pub fn try_add_gateway_on_behalf(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
    owner: String,
    owner_signature: String,
) -> Result<Response, ContractError> {
    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::CONTRACT_STATE
        .load(deps.storage)?
        .params
        .minimum_mixnode_pledge;
    let pledge = validate_gateway_pledge(info.funds, minimum_pledge)?;

    let proxy = info.sender;
    _try_add_gateway(
        deps,
        env,
        gateway,
        pledge,
        &owner,
        owner_signature,
        Some(proxy),
    )
}

pub(crate) fn _try_add_gateway(
    deps: DepsMut,
    env: Env,
    gateway: Gateway,
    pledge: Coin,
    owner: &str,
    owner_signature: String,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(owner)?;

    // if the client has an active bonded mixnode or gateway, don't allow bonding
    ensure_no_existing_bond(deps.storage, &owner)?;

    // check if somebody else has already bonded a gateway with this identity
    if let Some(existing_bond) =
        storage::gateways().may_load(deps.storage, &gateway.identity_key)?
    {
        if existing_bond.owner != owner {
            return Err(ContractError::DuplicateGateway {
                owner: existing_bond.owner,
            });
        }
    }

    // check if this sender actually owns the gateway by checking the signature
    validate_node_identity_signature(
        deps.as_ref(),
        &owner,
        owner_signature,
        &gateway.identity_key,
    )?;

    let gateway_identity = gateway.identity_key.clone();
    let bond = GatewayBond::new(
        pledge.clone(),
        owner.clone(),
        env.block.height,
        gateway,
        proxy.clone(),
    );

    storage::gateways().save(deps.storage, bond.identity(), &bond)?;
    mixnet_params_storage::increment_layer_count(deps.storage, Layer::Gateway)?;

    Ok(Response::new().add_event(new_gateway_bonding_event(
        &owner,
        &proxy,
        &pledge,
        &gateway_identity,
    )))
}

pub fn try_remove_gateway_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let proxy = info.sender;
    _try_remove_gateway(deps, &owner, Some(proxy))
}

pub fn try_remove_gateway(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    _try_remove_gateway(deps, info.sender.as_ref(), None)
}

pub(crate) fn _try_remove_gateway(
    deps: DepsMut,
    owner: &str,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(owner)?;
    // try to find the node of the sender
    let gateway_bond = match storage::gateways()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
    {
        Some(record) => record.1,
        None => return Err(ContractError::NoAssociatedGatewayBond { owner }),
    };

    if proxy != gateway_bond.proxy {
        return Err(ContractError::ProxyMismatch {
            existing: gateway_bond
                .proxy
                .map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
            incoming: proxy.map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
        });
    }

    // send bonded funds back to the bond owner
    let return_tokens = BankMsg::Send {
        to_address: proxy.as_ref().unwrap_or(&owner).to_string(),
        amount: vec![gateway_bond.pledge_amount()],
    };

    // remove the bond
    storage::gateways().remove(deps.storage, gateway_bond.identity())?;

    // decrement layer count
    mixnet_params_storage::decrement_layer_count(deps.storage, Layer::Gateway)?;

    let mut response = Response::new().add_message(return_tokens);

    if let Some(proxy) = &proxy {
        let msg = VestingContractExecuteMsg::TrackUnbondGateway {
            owner: owner.as_str().to_string(),
            amount: gateway_bond.pledge_amount(),
        };

        let track_unbond_message = wasm_execute(proxy, &msg, coins(0, DENOM))?;
        response = response.add_message(track_unbond_message);
    }

    Ok(response.add_event(new_gateway_unbonding_event(
        &owner,
        &proxy,
        &gateway_bond.pledge_amount,
        gateway_bond.identity(),
    )))
}

fn validate_gateway_pledge(
    mut pledge: Vec<Coin>,
    minimum_pledge: Uint128,
) -> Result<Coin, ContractError> {
    // check if anything was put as bond
    if pledge.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if pledge.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if pledge[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have at least 100 coins in our pledge
    if pledge[0].amount < minimum_pledge {
        return Err(ContractError::InsufficientGatewayBond {
            received: pledge[0].amount.into(),
            minimum: minimum_pledge.into(),
        });
    }

    Ok(pledge.pop().unwrap())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{execute, query, INITIAL_GATEWAY_PLEDGE};
    use crate::error::ContractError;
    use crate::gateways::transactions::validate_gateway_pledge;
    use crate::support::tests;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coins, BankMsg, Response};
    use cosmwasm_std::{from_binary, Addr, Uint128};
    use mixnet_contract_common::{ExecuteMsg, Gateway, PagedGatewayResponse, QueryMsg};

    #[test]
    fn gateway_add() {
        let mut deps = test_helpers::init_contract();

        // if we fail validation (by say not sending enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_GATEWAY_PLEDGE) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let (msg, _) = tests::messages::valid_bond_gateway_msg("anyone");

        // we are informed that we didn't send enough funds
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: insufficient_bond,
                minimum: INITIAL_GATEWAY_PLEDGE.into(),
            })
        );

        // make sure no gateway was inserted into the topology
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetGateways {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedGatewayResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len());

        // if we send enough funds
        let info = mock_info("anyone", &tests::fixtures::good_gateway_pledge());
        let (msg, identity) = tests::messages::valid_bond_gateway_msg("anyone");

        // we get back a message telling us everything was OK
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // we can query topology and the new node is there
        let query_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetGateways {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedGatewayResponse = from_binary(&query_response).unwrap();
        assert_eq!(1, page.nodes.len());
        assert_eq!(
            &Gateway {
                identity_key: identity,
                ..tests::fixtures::gateway_fixture()
            },
            page.nodes[0].gateway()
        );

        // if there was already a gateway bonded by particular user
        let info = mock_info("foomper", &tests::fixtures::good_gateway_pledge());
        let (msg, _) = tests::messages::valid_bond_gateway_msg("foomper");
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("foomper", &tests::fixtures::good_gateway_pledge());
        let (msg, _) = tests::messages::valid_bond_gateway_msg("foomper");

        // it fails
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(Err(ContractError::AlreadyOwnsGateway), execute_response);

        // bonding fails if the user already owns a mixnode
        test_helpers::add_mixnode(
            "mixnode-owner",
            tests::fixtures::good_mixnode_pledge(),
            deps.as_mut(),
        );

        let info = mock_info("mixnode-owner", &tests::fixtures::good_gateway_pledge());
        let (msg, _) = tests::messages::valid_bond_gateway_msg("mixnode-owner");

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsMixnode));

        // but after he unbonds it, it's all fine again
        let info = mock_info("mixnode-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &tests::fixtures::good_gateway_pledge());
        let (msg, _) = tests::messages::valid_bond_gateway_msg("mixnode-owner");

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state).
        // Is that right? Think about this, not sure yet.
    }

    #[test]
    fn adding_gateway_without_existing_owner() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("gateway-owner", &tests::fixtures::good_gateway_pledge());

        // before the execution the node had no associated owner
        assert!(storage::gateways()
            .idx
            .owner
            .item(deps.as_ref().storage, Addr::unchecked("gateway-owner"))
            .unwrap()
            .is_none());

        let (msg, identity) = tests::messages::valid_bond_gateway_msg("gateway-owner");

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            &identity,
            storage::gateways()
                .idx
                .owner
                .item(deps.as_ref().storage, Addr::unchecked("gateway-owner"))
                .unwrap()
                .unwrap()
                .1
                .identity()
        );
    }

    #[test]
    fn adding_gateway_with_existing_owner() {
        let mut deps = test_helpers::init_contract();

        let identity = test_helpers::add_gateway(
            "gateway-owner",
            tests::fixtures::good_gateway_pledge(),
            deps.as_mut(),
        );

        // request fails giving the existing owner address in the message
        let info = mock_info(
            "gateway-owner-pretender",
            &tests::fixtures::good_gateway_pledge(),
        );
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: identity,
                ..tests::fixtures::gateway_fixture()
            },
            owner_signature: "foomp".to_string(),
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            Err(ContractError::DuplicateGateway {
                owner: Addr::unchecked("gateway-owner")
            }),
            execute_response
        );
    }

    #[test]
    fn adding_gateway_with_existing_unchanged_owner() {
        let mut deps = test_helpers::init_contract();

        test_helpers::add_gateway(
            "gateway-owner",
            tests::fixtures::good_gateway_pledge(),
            deps.as_mut(),
        );

        let info = mock_info("gateway-owner", &tests::fixtures::good_gateway_pledge());
        let (msg, _) = tests::messages::valid_bond_gateway_msg("gateway-owner");

        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(Err(ContractError::AlreadyOwnsGateway), res);
    }

    #[test]
    fn gateway_remove() {
        let mut deps = test_helpers::init_contract();

        // try unbond when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("anyone")
            })
        );

        // let's add a node owned by bob
        test_helpers::add_gateway("bob", tests::fixtures::good_gateway_pledge(), deps.as_mut());

        // attempt to unbond fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("fred")
            })
        );

        // bob's node is still there
        let nodes = tests::queries::get_gateways(&mut deps);
        assert_eq!(1, nodes.len());

        let first_node = &nodes[0];
        assert_eq!(&Addr::unchecked("bob"), first_node.owner());

        // add a node owned by fred
        let fred_identity = test_helpers::add_gateway(
            "fred",
            tests::fixtures::good_gateway_pledge(),
            deps.as_mut(),
        );

        // let's make sure we now have 2 nodes:
        assert_eq!(2, tests::queries::get_gateways(&mut deps).len());

        // unbond fred's node
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let remove_fred = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see a funds transfer from the contract back to fred
        let expected_message = BankMsg::Send {
            to_address: String::from(info.sender),
            amount: tests::fixtures::good_gateway_pledge(),
        };

        // run the executor and check that we got back the correct results
        let expected_response =
            Response::new()
                .add_message(expected_message)
                .add_event(new_gateway_unbonding_event(
                    &Addr::unchecked("fred"),
                    &None,
                    &tests::fixtures::good_gateway_pledge()[0],
                    &fred_identity,
                ));

        assert_eq!(expected_response, remove_fred);

        // only 1 node now exists, owned by bob:
        let gateway_bonds = tests::queries::get_gateways(&mut deps);
        assert_eq!(1, gateway_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), gateway_bonds[0].owner());
    }

    #[test]
    fn removing_gateway_clears_ownership() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("gateway-owner", &tests::fixtures::good_gateway_pledge());
        let (bond_msg, identity) = tests::messages::valid_bond_gateway_msg("gateway-owner");
        execute(deps.as_mut(), mock_env(), info, bond_msg.clone()).unwrap();

        assert_eq!(
            &identity,
            storage::gateways()
                .idx
                .owner
                .item(deps.as_ref().storage, Addr::unchecked("gateway-owner"))
                .unwrap()
                .unwrap()
                .1
                .identity()
        );

        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(storage::gateways()
            .idx
            .owner
            .item(deps.as_ref().storage, Addr::unchecked("gateway-owner"))
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("gateway-owner", &tests::fixtures::good_gateway_pledge());

        assert!(execute(deps.as_mut(), mock_env(), info, bond_msg).is_ok());
        assert_eq!(
            &identity,
            storage::gateways()
                .idx
                .owner
                .item(deps.as_ref().storage, Addr::unchecked("gateway-owner"))
                .unwrap()
                .unwrap()
                .1
                .identity()
        );
    }

    #[test]
    fn validating_gateway_bond() {
        // you must send SOME funds
        let result = validate_gateway_pledge(Vec::new(), INITIAL_GATEWAY_PLEDGE);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = tests::fixtures::good_gateway_pledge();
        bond[0].amount = INITIAL_GATEWAY_PLEDGE.checked_sub(Uint128::new(1)).unwrap();
        let result = validate_gateway_pledge(bond.clone(), INITIAL_GATEWAY_PLEDGE);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: Into::<u128>::into(INITIAL_GATEWAY_PLEDGE) - 1,
                minimum: INITIAL_GATEWAY_PLEDGE.into(),
            })
        );

        // more than that is still fine
        let mut bond = tests::fixtures::good_gateway_pledge();
        bond[0].amount = INITIAL_GATEWAY_PLEDGE + Uint128::new(1);
        let result = validate_gateway_pledge(bond.clone(), INITIAL_GATEWAY_PLEDGE);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = tests::fixtures::good_gateway_pledge();
        bond[0].denom = "baddenom".to_string();
        let result = validate_gateway_pledge(bond.clone(), INITIAL_GATEWAY_PLEDGE);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = tests::fixtures::good_gateway_pledge();
        bond[0].denom = "foomp".to_string();
        let result = validate_gateway_pledge(bond.clone(), INITIAL_GATEWAY_PLEDGE);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }
}
