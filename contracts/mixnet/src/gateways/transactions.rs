// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::support::helpers::{
    ensure_no_existing_bond, ensure_sent_by_vesting_contract, validate_node_identity_signature,
    validate_pledge,
};
use cosmwasm_std::{wasm_execute, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{new_gateway_bonding_event, new_gateway_unbonding_event};
use mixnet_contract_common::{Gateway, GatewayBond};
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

pub fn try_add_gateway(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
    owner_signature: String,
) -> Result<Response, MixnetContractError> {
    _try_add_gateway(
        deps,
        env,
        gateway,
        info.funds,
        info.sender,
        owner_signature,
        None,
    )
}

pub fn try_add_gateway_on_behalf(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
    owner: String,
    owner_signature: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_add_gateway(
        deps,
        env,
        gateway,
        info.funds,
        owner,
        owner_signature,
        Some(proxy),
    )
}

pub(crate) fn _try_add_gateway(
    deps: DepsMut<'_>,
    env: Env,
    gateway: Gateway,
    pledge: Vec<Coin>,
    owner: Addr,
    owner_signature: String,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::minimum_gateway_pledge(deps.storage)?;
    let pledge = validate_pledge(pledge, minimum_pledge)?;

    // if the client has an active bonded mixnode or gateway, don't allow bonding
    ensure_no_existing_bond(&owner, deps.storage)?;

    // check if somebody else has already bonded a gateway with this identity
    if let Some(existing_bond) =
        storage::gateways().may_load(deps.storage, &gateway.identity_key)?
    {
        if existing_bond.owner != owner {
            return Err(MixnetContractError::DuplicateGateway {
                owner: existing_bond.owner,
            });
        }
    }

    // check if this sender actually owns the gateway by checking the signature
    validate_node_identity_signature(
        deps.as_ref(),
        &owner,
        &owner_signature,
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

    Ok(Response::new().add_event(new_gateway_bonding_event(
        &owner,
        &proxy,
        &pledge,
        &gateway_identity,
    )))
}

pub fn try_remove_gateway_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    ensure_sent_by_vesting_contract(&info, deps.storage)?;

    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_remove_gateway(deps, owner, Some(proxy))
}

pub fn try_remove_gateway(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_remove_gateway(deps, info.sender, None)
}

pub(crate) fn _try_remove_gateway(
    deps: DepsMut<'_>,
    owner: Addr,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // try to find the node of the sender
    let gateway_bond = match storage::gateways()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
    {
        Some(record) => record.1,
        None => return Err(MixnetContractError::NoAssociatedGatewayBond { owner }),
    };

    if proxy != gateway_bond.proxy {
        return Err(MixnetContractError::ProxyMismatch {
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

    let mut response = Response::new().add_message(return_tokens);

    if let Some(proxy) = &proxy {
        let msg = VestingContractExecuteMsg::TrackUnbondGateway {
            owner: owner.as_str().to_string(),
            amount: gateway_bond.pledge_amount(),
        };

        let track_unbond_message = wasm_execute(proxy, &msg, vec![])?;
        response = response.add_message(track_unbond_message);
    }

    Ok(response.add_event(new_gateway_unbonding_event(
        &owner,
        &proxy,
        &gateway_bond.pledge_amount,
        gateway_bond.identity(),
    )))
}

#[cfg(test)]
pub mod tests {
    use crate::contract::execute;
    use crate::gateways::transactions::{
        try_add_gateway, try_add_gateway_on_behalf, try_remove_gateway_on_behalf,
    };
    use crate::interval::pending_events;
    use crate::mixnet_contract_settings::storage::minimum_gateway_pledge;
    use crate::support::tests;
    use crate::support::tests::fixtures::{good_gateway_pledge, TEST_COIN_DENOM};
    use crate::support::tests::test_helpers::TestSetup;
    use crate::support::tests::{fixtures, test_helpers};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Addr, BankMsg, Response, Uint128};
    use mixnet_contract_common::error::MixnetContractError;
    use mixnet_contract_common::events::new_gateway_unbonding_event;
    use mixnet_contract_common::ExecuteMsg;

    #[test]
    fn gateway_add() {
        let mut deps = test_helpers::init_contract();
        let env = mock_env();
        let mut rng = test_helpers::test_rng();

        // if we fail validation (by say not sending enough funds
        let sender = "alice";
        let minimum_pledge = minimum_gateway_pledge(deps.as_ref().storage).unwrap();
        let mut insufficient_pledge = minimum_pledge.clone();
        insufficient_pledge.amount -= Uint128::new(1000);

        let info = mock_info(sender, &[insufficient_pledge.clone()]);
        let (gateway, sig) = test_helpers::gateway_with_signature(&mut rng, sender);

        let result = try_add_gateway(
            deps.as_mut(),
            env.clone(),
            info,
            gateway.clone(),
            sig.clone(),
        );

        // we are informed that we didn't send enough funds
        assert_eq!(
            result,
            Err(MixnetContractError::InsufficientPledge {
                received: insufficient_pledge,
                minimum: minimum_pledge.clone(),
            })
        );

        // if the signature provided is invalid, the bonding also fails
        let info = mock_info(sender, &[minimum_pledge]);

        let result = try_add_gateway(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            gateway.clone(),
            "bad-signature".into(),
        );
        assert!(matches!(
            result,
            Err(MixnetContractError::MalformedEd25519Signature(..))
        ));

        // if there was already a gateway bonded by particular user
        test_helpers::add_gateway(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            sender,
            fixtures::good_gateway_pledge(),
        );

        // it fails
        let result = try_add_gateway(deps.as_mut(), env.clone(), info, gateway, sig);
        assert_eq!(Err(MixnetContractError::AlreadyOwnsGateway), result);

        // the same holds if the user already owns a mixnode
        let sender2 = "mixnode-owner";

        let mix_id = test_helpers::add_mixnode(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            sender2,
            vec![coin(100_000_000, TEST_COIN_DENOM)],
        );

        let info = mock_info(sender2, &fixtures::good_gateway_pledge());
        let (gateway, sig) = test_helpers::gateway_with_signature(&mut rng, sender2);

        let result = try_add_gateway(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            gateway.clone(),
            sig.clone(),
        );
        assert_eq!(Err(MixnetContractError::AlreadyOwnsMixnode), result);

        // but after he unbonds it, it's all fine again
        pending_events::unbond_mixnode(deps.as_mut(), &env, 123, mix_id).unwrap();

        let result = try_add_gateway(deps.as_mut(), env, info, gateway, sig);
        assert!(result.is_ok());
    }

    #[test]
    fn gateway_add_with_illegal_proxy() {
        let mut test = TestSetup::new();
        let env = test.env();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";
        let (gateway, sig) = test_helpers::gateway_with_signature(&mut test.rng, owner);

        let res = try_add_gateway_on_behalf(
            test.deps_mut(),
            env,
            mock_info(illegal_proxy.as_ref(), &good_gateway_pledge()),
            gateway,
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
    fn gateway_remove() {
        let mut deps = test_helpers::init_contract();
        let mut rng = test_helpers::test_rng();
        let env = mock_env();

        // try unbond when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(MixnetContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("anyone")
            })
        );

        // let's add a node owned by bob
        test_helpers::add_gateway(
            &mut rng,
            deps.as_mut(),
            env.clone(),
            "bob",
            fixtures::good_gateway_pledge(),
        );

        // attempt to unbond fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(MixnetContractError::NoAssociatedGatewayBond {
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
            &mut rng,
            deps.as_mut(),
            env,
            "fred",
            tests::fixtures::good_gateway_pledge(),
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
    fn gateway_remove_with_illegal_proxy() {
        let mut test = TestSetup::new();

        let illegal_proxy = Addr::unchecked("not-vesting-contract");
        let vesting_contract = test.vesting_contract();

        let owner = "alice";

        test.add_dummy_gateway_with_illegal_proxy(owner, None, illegal_proxy.clone());

        let res = try_remove_gateway_on_behalf(
            test.deps_mut(),
            mock_info(illegal_proxy.as_ref(), &good_gateway_pledge()),
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
