use super::storage;
use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use config::defaults::DENOM;
use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128};
use mixnet_contract::{Gateway, GatewayBond, Layer};

pub(crate) fn try_add_gateway(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // if the client has an active bonded mixnode, don't allow gateway bonding
    if mixnodes_storage::mixnodes()
        .idx
        .owner
        .item(deps.storage, info.sender.clone())?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsMixnode);
    }

    // if the client has an active bonded gateway, regardless of its identity, don't allow bonding
    if storage::gateways_owners_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some()
    {
        return Err(ContractError::AlreadyOwnsGateway);
    }

    // check if somebody else has already bonded a gateway with this identity
    if let Some(existing_bond) =
        storage::gateways_read(deps.storage).may_load(gateway.identity_key.as_bytes())?
    {
        if existing_bond.owner != info.sender {
            return Err(ContractError::DuplicateGateway {
                owner: existing_bond.owner,
            });
        }
    }

    let minimum_bond = mixnet_params_storage::CONTRACT_SETTINGS
        .load(deps.storage)?
        .params
        .minimum_gateway_bond;
    validate_gateway_bond(&info.funds, minimum_bond)?;

    let bond = GatewayBond::new(
        info.funds[0].clone(),
        info.sender.clone(),
        env.block.height,
        gateway,
    );

    let identity = bond.identity();
    storage::gateways(deps.storage).save(identity.as_bytes(), &bond)?;
    storage::gateways_owners(deps.storage).save(sender_bytes, identity)?;
    mixnet_params_storage::increment_layer_count(deps.storage, Layer::Gateway)?;

    Ok(Response::new())
}

pub(crate) fn try_remove_gateway(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender_bytes = info.sender.as_str().as_bytes();

    // try to find the identity of the sender's node
    let gateway_identity =
        match storage::gateways_owners_read(deps.storage).may_load(sender_bytes)? {
            Some(identity) => identity,
            None => return Err(ContractError::NoAssociatedGatewayBond { owner: info.sender }),
        };

    // get the bond, since we found associated identity, the node MUST exist
    let gateway_bond = storage::gateways_read(deps.storage).load(gateway_identity.as_bytes())?;

    // send bonded funds back to the bond owner
    let return_tokens = BankMsg::Send {
        to_address: info.sender.as_str().to_owned(),
        amount: vec![gateway_bond.bond_amount()],
    };

    // remove the bond from the list of bonded gateways
    storage::gateways(deps.storage).remove(gateway_identity.as_bytes());
    // remove the node ownership
    storage::gateways_owners(deps.storage).remove(sender_bytes);
    // decrement layer count
    mixnet_params_storage::decrement_layer_count(deps.storage, Layer::Gateway)?;

    Ok(Response::new()
        .add_message(return_tokens)
        .add_attribute("action", "unbond")
        .add_attribute("address", info.sender)
        .add_attribute("gateway_bond", gateway_bond.to_string()))
}

fn validate_gateway_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
    // check if anything was put as bond
    if bond.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if bond.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if bond[0].denom != DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have at least 100 coins in our bond
    if bond[0].amount < minimum_bond {
        return Err(ContractError::InsufficientGatewayBond {
            received: bond[0].amount.into(),
            minimum: minimum_bond.into(),
        });
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{execute, query, INITIAL_GATEWAY_BOND};
    use crate::error::ContractError;
    use crate::gateways::transactions::try_add_gateway;
    use crate::gateways::transactions::validate_gateway_bond;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coins, BankMsg, Response};
    use cosmwasm_std::{from_binary, Addr, Uint128};
    use mixnet_contract::Gateway;
    use mixnet_contract::MixNode;
    use mixnet_contract::{ExecuteMsg, PagedGatewayResponse, QueryMsg};

    #[test]
    fn gateway_add() {
        let mut deps = test_helpers::init_contract();

        // if we fail validation (by say not sending enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_GATEWAY_BOND) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let msg = ExecuteMsg::BondGateway {
            gateway: test_helpers::gateway_fixture(),
        };

        // we are informed that we didn't send enough funds
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: insufficient_bond,
                minimum: INITIAL_GATEWAY_BOND.into(),
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
        let info = mock_info("anyone", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "anyonesgateway".into(),
                ..test_helpers::gateway_fixture()
            },
        };

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
                identity_key: "anyonesgateway".into(),
                ..test_helpers::gateway_fixture()
            },
            page.nodes[0].gateway()
        );

        // if there was already a gateway bonded by particular user
        let info = mock_info("foomper", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "foompersgateway".into(),
                ..test_helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("foomper", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "foompersgateway".into(),
                ..test_helpers::gateway_fixture()
            },
        };

        // it fails
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(Err(ContractError::AlreadyOwnsGateway), execute_response);

        // bonding fails if the user already owns a mixnode
        let info = mock_info("mixnode-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmix".into(),
                ..test_helpers::mix_node_fixture()
            },
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: test_helpers::gateway_fixture(),
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsMixnode));

        // but after he unbonds it, it's all fine again
        let info = mock_info("mixnode-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: test_helpers::gateway_fixture(),
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state).
        // Is that right? Think about this, not sure yet.
    }

    #[test]
    fn adding_gateway_without_existing_owner() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("gateway-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..test_helpers::gateway_fixture()
            },
        };

        // before the execution the node had no associated owner
        assert!(storage::gateways_owners_read(deps.as_ref().storage)
            .may_load("gateway-owner".as_bytes())
            .unwrap()
            .is_none());

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            "myAwesomeGateway",
            storage::gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn adding_gateway_with_existing_owner() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("gateway-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..test_helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // request fails giving the existing owner address in the message
        let info = mock_info(
            "gateway-owner-pretender",
            &test_helpers::good_gateway_bond(),
        );
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..test_helpers::gateway_fixture()
            },
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

        let info = mock_info("gateway-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                host: "1.1.1.1".into(),
                ..test_helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                host: "2.2.2.2".into(),
                ..test_helpers::gateway_fixture()
            },
        };

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
        test_helpers::add_gateway("bob", test_helpers::good_gateway_bond(), &mut deps);

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
        let nodes = test_helpers::get_gateways(&mut deps);
        assert_eq!(1, nodes.len());

        let first_node = &nodes[0];
        assert_eq!(&Addr::unchecked("bob"), first_node.owner());

        // add a node owned by fred
        let info = mock_info("fred", &test_helpers::good_gateway_bond());
        try_add_gateway(
            deps.as_mut(),
            mock_env(),
            info,
            Gateway {
                identity_key: "fredsgateway".into(),
                ..test_helpers::gateway_fixture()
            },
        )
        .unwrap();

        // let's make sure we now have 2 nodes:
        assert_eq!(2, test_helpers::get_gateways(&mut deps).len());

        // unbond fred's node
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        let remove_fred = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr("address", "fred"),
            attr(
                "gateway_bond",
                format!(
                    "amount: {} {}, owner: fred, identity: fredsgateway",
                    INITIAL_GATEWAY_BOND, DENOM
                ),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_message = BankMsg::Send {
            to_address: String::from(info.sender),
            amount: test_helpers::good_gateway_bond(),
        };

        // run the executor and check that we got back the correct results
        let expected = Response::new()
            .add_attributes(expected_attributes)
            .add_message(expected_message);

        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let gateway_bonds = test_helpers::get_gateways(&mut deps);
        assert_eq!(1, gateway_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), gateway_bonds[0].owner());
    }

    #[test]
    fn removing_gateway_clears_ownership() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("gateway-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..test_helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            "myAwesomeGateway",
            storage::gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );

        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(storage::gateways_owners_read(deps.as_ref().storage)
            .may_load("gateway-owner".as_bytes())
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("gateway-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..test_helpers::gateway_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());
        assert_eq!(
            "myAwesomeGateway",
            storage::gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn validating_gateway_bond() {
        // you must send SOME funds
        let result = validate_gateway_bond(&[], INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = test_helpers::good_gateway_bond();
        bond[0].amount = INITIAL_GATEWAY_BOND.checked_sub(Uint128::new(1)).unwrap();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: Into::<u128>::into(INITIAL_GATEWAY_BOND) - 1,
                minimum: INITIAL_GATEWAY_BOND.into(),
            })
        );

        // more than that is still fine
        let mut bond = test_helpers::good_gateway_bond();
        bond[0].amount = INITIAL_GATEWAY_BOND + Uint128::new(1);
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = test_helpers::good_gateway_bond();
        bond[0].denom = "baddenom".to_string();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = test_helpers::good_gateway_bond();
        bond[0].denom = "foomp".to_string();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }
}
