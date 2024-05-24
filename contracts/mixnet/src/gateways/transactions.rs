// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::helpers::must_get_gateway_bond_by_owner;
use super::storage;
use crate::gateways::signature_helpers::verify_gateway_bonding_signature;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::signing::storage as signing_storage;
use crate::support::helpers::{ensure_no_existing_bond, validate_pledge};
use cosmwasm_std::{BankMsg, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_gateway_bonding_event, new_gateway_config_update_event, new_gateway_unbonding_event,
};
use mixnet_contract_common::gateway::GatewayConfigUpdate;
use mixnet_contract_common::{Gateway, GatewayBond};
use nym_contracts_common::signing::MessageSignature;

// TODO: perhaps also require the user to explicitly provide what it thinks is the current nonce
// so that we could return a better error message if it doesn't match?
pub(crate) fn try_add_gateway(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
    owner_signature: MessageSignature,
) -> Result<Response, MixnetContractError> {
    // check if the pledge contains any funds of the appropriate denomination
    let minimum_pledge = mixnet_params_storage::minimum_gateway_pledge(deps.storage)?;
    let pledge = validate_pledge(info.funds, minimum_pledge)?;

    // if the client has an active bonded mixnode or gateway, don't allow bonding
    ensure_no_existing_bond(&info.sender, deps.storage)?;

    // check if somebody else has already bonded a gateway with this identity
    if let Some(existing_bond) =
        storage::gateways().may_load(deps.storage, &gateway.identity_key)?
    {
        if existing_bond.owner != info.sender {
            return Err(MixnetContractError::DuplicateGateway {
                owner: existing_bond.owner,
            });
        }
    }

    // check if this sender actually owns the gateway by checking the signature
    verify_gateway_bonding_signature(
        deps.as_ref(),
        info.sender.clone(),
        pledge.clone(),
        gateway.clone(),
        owner_signature,
    )?;

    // update the signing nonce associated with this sender so that the future signature would be made on the new value
    signing_storage::increment_signing_nonce(deps.storage, info.sender.clone())?;

    let gateway_identity = gateway.identity_key.clone();
    let bond = GatewayBond::new(
        pledge.clone(),
        info.sender.clone(),
        env.block.height,
        gateway,
    );

    storage::gateways().save(deps.storage, bond.identity(), &bond)?;

    Ok(Response::new().add_event(new_gateway_bonding_event(
        &info.sender,
        &pledge,
        &gateway_identity,
    )))
}

pub(crate) fn try_remove_gateway(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // try to find the node of the sender
    let gateway_bond = match storage::gateways()
        .idx
        .owner
        .item(deps.storage, info.sender.clone())?
    {
        Some(record) => record.1,
        None => return Err(MixnetContractError::NoAssociatedGatewayBond { owner: info.sender }),
    };

    // send bonded funds back to the bond owner
    let return_tokens = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![gateway_bond.pledge_amount()],
    };

    // remove the bond
    storage::gateways().remove(deps.storage, gateway_bond.identity())?;

    Ok(Response::new()
        .add_message(return_tokens)
        .add_event(new_gateway_unbonding_event(
            &info.sender,
            &gateway_bond.pledge_amount,
            gateway_bond.identity(),
        )))
}

pub(crate) fn try_update_gateway_config(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_config: GatewayConfigUpdate,
) -> Result<Response, MixnetContractError> {
    let existing_bond = must_get_gateway_bond_by_owner(deps.storage, &info.sender)?;
    let cfg_update_event = new_gateway_config_update_event(&info.sender, &new_config);

    let mut updated_bond = existing_bond.clone();
    updated_bond.gateway.host = new_config.host;
    updated_bond.gateway.mix_port = new_config.mix_port;
    updated_bond.gateway.clients_port = new_config.clients_port;
    updated_bond.gateway.location = new_config.location;
    updated_bond.gateway.version = new_config.version;

    storage::gateways().replace(
        deps.storage,
        existing_bond.identity(),
        Some(&updated_bond),
        Some(&existing_bond),
    )?;

    Ok(Response::new().add_event(cfg_update_event))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::gateways::queries;
    use crate::interval::pending_events;
    use crate::mixnet_contract_settings::storage::minimum_gateway_pledge;
    use crate::support::tests;
    use crate::support::tests::fixtures;
    use crate::support::tests::fixtures::good_mixnode_pledge;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{Addr, Uint128};
    use mixnet_contract_common::ExecuteMsg;

    #[test]
    fn gateway_add() {
        let mut test = TestSetup::new();

        // if we fail validation (by say not sending enough funds
        let sender = "alice";
        let minimum_pledge = minimum_gateway_pledge(test.deps().storage).unwrap();
        let mut insufficient_pledge = minimum_pledge.clone();
        insufficient_pledge.amount -= Uint128::new(1000);

        let info = mock_info(sender, &[insufficient_pledge.clone()]);
        let (gateway, sig) =
            test.gateway_with_signature(sender, Some(vec![insufficient_pledge.clone()]));

        let env = test.env();
        let result = try_add_gateway(
            test.deps_mut(),
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

        // if there was already a gateway bonded by particular user
        test.add_dummy_gateway(sender, None);

        // it fails
        let result = try_add_gateway(test.deps_mut(), env.clone(), info, gateway, sig);
        assert_eq!(Err(MixnetContractError::AlreadyOwnsGateway), result);

        // the same holds if the user already owns a mixnode
        let sender2 = "mixnode-owner";

        let mix_id = test.add_dummy_mixnode(sender2, None);

        let info = mock_info(sender2, &fixtures::good_gateway_pledge());
        let (gateway, sig) = test.gateway_with_signature(sender2, None);

        let result = try_add_gateway(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            gateway.clone(),
            sig.clone(),
        );
        assert_eq!(Err(MixnetContractError::AlreadyOwnsMixnode), result);

        // but after he unbonds it, it's all fine again
        pending_events::unbond_mixnode(test.deps_mut(), &env, 123, mix_id).unwrap();

        let result = try_add_gateway(test.deps_mut(), env, info, gateway, sig);
        assert!(result.is_ok());
    }

    #[test]
    fn adding_gateway_with_invalid_signatures() {
        let mut test = TestSetup::new();
        let env = test.env();

        let sender = "alice";
        let pledge = good_mixnode_pledge();
        let info = mock_info(sender, pledge.as_ref());

        let (gateway, signature) = test.gateway_with_signature(sender, Some(pledge.clone()));

        // using different parameters than what the signature was made on
        let mut modified_gateway = gateway.clone();
        modified_gateway.mix_port += 1;
        let res = try_add_gateway(
            test.deps_mut(),
            env.clone(),
            info,
            modified_gateway,
            signature.clone(),
        );
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));

        // even stake amount is protected
        let mut different_pledge = pledge.clone();
        different_pledge[0].amount += Uint128::new(12345);

        let info = mock_info(sender, different_pledge.as_ref());
        let res = try_add_gateway(
            test.deps_mut(),
            env.clone(),
            info,
            gateway.clone(),
            signature.clone(),
        );
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));

        let other_sender = mock_info("another-sender", pledge.as_ref());
        let res = try_add_gateway(
            test.deps_mut(),
            env.clone(),
            other_sender,
            gateway.clone(),
            signature.clone(),
        );
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));

        // trying to reuse the same signature for another bonding fails (because nonce doesn't match!)
        let info = mock_info(sender, pledge.as_ref());
        let current_nonce =
            signing_storage::get_signing_nonce(test.deps().storage, Addr::unchecked(sender))
                .unwrap();
        assert_eq!(0, current_nonce);
        let res = try_add_gateway(
            test.deps_mut(),
            env.clone(),
            info.clone(),
            gateway.clone(),
            signature.clone(),
        );
        assert!(res.is_ok());
        let updated_nonce =
            signing_storage::get_signing_nonce(test.deps().storage, Addr::unchecked(sender))
                .unwrap();
        assert_eq!(1, updated_nonce);

        try_remove_gateway(test.deps_mut(), info.clone()).unwrap();

        let res = try_add_gateway(test.deps_mut(), env, info, gateway, signature);
        assert_eq!(res, Err(MixnetContractError::InvalidEd25519Signature));
    }

    #[test]
    fn gateway_remove() {
        let mut test = TestSetup::new();
        let env = test.env();

        // try unbond when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(test.deps_mut(), env.clone(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(MixnetContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("anyone")
            })
        );

        // let's add a node owned by bob
        test.add_dummy_gateway("bob", None);

        // attempt to unbond fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let result = execute(test.deps_mut(), env.clone(), info, msg);
        assert_eq!(
            result,
            Err(MixnetContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked("fred")
            })
        );

        // bob's node is still there
        let nodes = queries::query_gateways_paged(test.deps(), None, None)
            .unwrap()
            .nodes;
        assert_eq!(1, nodes.len());

        let first_node = &nodes[0];
        assert_eq!(&Addr::unchecked("bob"), first_node.owner());

        // add a node owned by fred
        let fred_identity = test.add_dummy_gateway("fred", None);

        // let's make sure we now have 2 nodes:
        let nodes = queries::query_gateways_paged(test.deps(), None, None)
            .unwrap()
            .nodes;
        assert_eq!(2, nodes.len());

        // unbond fred's node
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let remove_fred = execute(test.deps_mut(), env, info.clone(), msg).unwrap();

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
                    &tests::fixtures::good_gateway_pledge()[0],
                    &fred_identity,
                ));

        assert_eq!(expected_response, remove_fred);

        // only 1 node now exists, owned by bob:
        let nodes = queries::query_gateways_paged(test.deps(), None, None)
            .unwrap()
            .nodes;
        assert_eq!(1, nodes.len());
        assert_eq!(&Addr::unchecked("bob"), nodes[0].owner());
    }

    #[test]
    fn update_gateway_config() {
        let mut test = TestSetup::new();

        let owner = "alice";
        let info = mock_info(owner, &[]);
        let update = GatewayConfigUpdate {
            host: "1.1.1.1:1234".to_string(),
            mix_port: 1234,
            clients_port: 1235,
            location: "home".to_string(),
            version: "v1.2.3".to_string(),
        };

        // try updating a non existing gateway bond
        let res = try_update_gateway_config(test.deps_mut(), info.clone(), update.clone());
        assert_eq!(
            res,
            Err(MixnetContractError::NoAssociatedGatewayBond {
                owner: Addr::unchecked(owner)
            })
        );

        test.add_dummy_gateway(owner, None);

        // "normal" update succeeds
        let res = try_update_gateway_config(test.deps_mut(), info, update.clone());
        assert!(res.is_ok());

        // and the config has actually been updated
        let bond =
            must_get_gateway_bond_by_owner(test.deps().storage, &Addr::unchecked(owner)).unwrap();
        assert_eq!(bond.gateway.host, update.host);
        assert_eq!(bond.gateway.mix_port, update.mix_port);
        assert_eq!(bond.gateway.clients_port, update.clients_port);
        assert_eq!(bond.gateway.location, update.location);
        assert_eq!(bond.gateway.version, update.version);
    }
}
