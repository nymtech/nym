// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::layer_queries::query_layer_distribution;
use crate::mixnodes::storage::StoredMixnodeBond;
use crate::support::helpers::ensure_no_existing_bond;
use config::defaults::DENOM;
use cosmwasm_std::{
    coins, wasm_execute, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Uint128,
};
use mixnet_contract::MixNode;
use vesting_contract::messages::ExecuteMsg as VestingContractExecuteMsg;

pub fn try_add_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
) -> Result<Response, ContractError> {
    // check if the bond contains any funds of the appropriate denomination
    let minimum_bond = mixnet_params_storage::CONTRACT_STATE
        .load(deps.storage)?
        .params
        .minimum_mixnode_bond;
    let bond = validate_mixnode_bond(info.funds, minimum_bond)?;

    _try_add_mixnode(deps, env, mix_node, bond, info.sender.as_str(), None)
}

pub fn try_add_mixnode_on_behalf(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_node: MixNode,
    owner: String,
) -> Result<Response, ContractError> {
    // check if the bond contains any funds of the appropriate denomination
    let minimum_bond = mixnet_params_storage::CONTRACT_STATE
        .load(deps.storage)?
        .params
        .minimum_mixnode_bond;
    let bond = validate_mixnode_bond(info.funds, minimum_bond)?;

    let proxy = info.sender.to_owned();
    _try_add_mixnode(deps, env, mix_node, bond, &owner, Some(proxy))
}

fn _try_add_mixnode(
    deps: DepsMut,
    env: Env,
    mix_node: MixNode,
    bond_amount: Coin,
    owner: &str,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(owner)?;
    // if the client has an active bonded mixnode or gateway, don't allow bonding
    ensure_no_existing_bond(deps.storage, &owner)?;

    // check if somebody else has already bonded a mixnode with this identity
    if let Some(existing_bond) =
        storage::mixnodes().may_load(deps.storage, &mix_node.identity_key)?
    {
        if existing_bond.owner != owner {
            return Err(ContractError::DuplicateMixnode {
                owner: existing_bond.owner,
            });
        }
    }

    let layer_distribution = query_layer_distribution(deps.as_ref())?;
    let layer = layer_distribution.choose_with_fewest();

    let stored_bond = StoredMixnodeBond::new(
        bond_amount,
        owner,
        layer,
        env.block.height,
        mix_node,
        None,
        proxy,
    );

    // technically we don't have to set the total_delegation bucket, but it makes things easier
    // in different places that we can guarantee that if node exists, so does the data behind the total delegation
    let identity = stored_bond.identity();
    storage::mixnodes().save(deps.storage, identity, &stored_bond)?;
    storage::TOTAL_DELEGATION.save(deps.storage, identity, &Uint128::zero())?;
    mixnet_params_storage::increment_layer_count(deps.storage, stored_bond.layer)?;

    Ok(Response::new())
}

pub fn try_remove_mixnode_on_behalf(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let proxy = info.sender;
    _try_remove_mixnode(deps, &owner, Some(proxy))
}

pub fn try_remove_mixnode(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    _try_remove_mixnode(deps, info.sender.as_ref(), None)
}

pub(crate) fn _try_remove_mixnode(
    deps: DepsMut,
    owner: &str,
    proxy: Option<Addr>,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(owner)?;

    // try to find the node of the sender
    let mixnode_bond = match storage::mixnodes()
        .idx
        .owner
        .item(deps.storage, owner.clone())?
    {
        Some(record) => record.1,
        None => return Err(ContractError::NoAssociatedMixNodeBond { owner }),
    };

    if proxy != mixnode_bond.proxy {
        return Err(ContractError::ProxyMismatch {
            existing: mixnode_bond
                .proxy
                .map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
            incoming: proxy.map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
        });
    }
    // send bonded funds back to the bond owner
    let return_tokens = BankMsg::Send {
        to_address: proxy.as_ref().unwrap_or(&owner).to_string(),
        amount: vec![mixnode_bond.bond_amount()],
    };

    // remove the bond
    storage::mixnodes().remove(deps.storage, mixnode_bond.identity())?;

    // decrement layer count
    mixnet_params_storage::decrement_layer_count(deps.storage, mixnode_bond.layer)?;

    let mut response = Response::new()
        .add_message(return_tokens)
        .add_attribute("action", "unbond")
        .add_attribute("mixnode_bond", mixnode_bond.to_string());

    if let Some(proxy) = &proxy {
        let msg = VestingContractExecuteMsg::TrackUnbond {
            owner: owner.as_str().to_string(),
            amount: mixnode_bond.bond_amount,
        };

        let track_unbond_message = wasm_execute(proxy, &msg, coins(0, DENOM))?;
        response = response.add_message(track_unbond_message);
    }

    Ok(response)
}

fn validate_mixnode_bond(
    mut bond: Vec<Coin>,
    minimum_bond: Uint128,
) -> Result<Coin, ContractError> {
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

    // check that we have at least MIXNODE_BOND coins in our bond
    if bond[0].amount < minimum_bond {
        return Err(ContractError::InsufficientMixNodeBond {
            received: bond[0].amount.into(),
            minimum: minimum_bond.into(),
        });
    }

    Ok(bond.pop().unwrap())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{execute, query, INITIAL_MIXNODE_BOND};
    use crate::error::ContractError;
    use crate::mixnodes::transactions::try_add_mixnode;
    use crate::mixnodes::transactions::validate_mixnode_bond;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coins, BankMsg, Response};
    use cosmwasm_std::{from_binary, Addr, Uint128};
    use mixnet_contract::Gateway;
    use mixnet_contract::Layer;
    use mixnet_contract::MixNode;
    use mixnet_contract::{ExecuteMsg, LayerDistribution, PagedMixnodeResponse, QueryMsg};

    #[test]
    fn mixnode_add() {
        let mut deps = test_helpers::init_contract();

        // if we don't send enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_MIXNODE_BOND) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "anyonesmixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
        };

        // we are informed that we didn't send enough funds
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientMixNodeBond {
                received: insufficient_bond,
                minimum: INITIAL_MIXNODE_BOND.into(),
            })
        );

        // no mixnode was inserted into the topology
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedMixnodeResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len());

        // if we send enough funds
        let info = mock_info("anyone", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "anyonesmixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
        };

        // we get back a message telling us everything was OK
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // we can query topology and the new node is there
        let query_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedMixnodeResponse = from_binary(&query_response).unwrap();
        assert_eq!(1, page.nodes.len());
        assert_eq!(
            &MixNode {
                identity_key: "anyonesmixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
            page.nodes[0].mix_node()
        );

        // if there was already a mixnode bonded by particular user
        let info = mock_info("foomper", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "foompermixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("foomper", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "foompermixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
        };

        // it fails
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(Err(ContractError::AlreadyOwnsMixnode), execute_response);

        // bonding fails if the user already owns a gateway
        let info = mock_info("gateway-owner", &test_helpers::good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "ownersgateway".into(),
                ..test_helpers::gateway_fixture()
            },
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsGateway));

        // but after he unbonds it, it's all fine again
        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmixnode".into(),
                ..test_helpers::mix_node_fixture()
            },
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state). Is that right? Think about this, not sure yet.
        // if we attempt to register a second node from the same address, should we get an error? It would probably be polite.
    }

    #[test]
    fn adding_mixnode_without_existing_owner() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        };

        // before the execution the node had no associated owner
        assert!(storage::mixnodes()
            .idx
            .owner
            .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
            .unwrap()
            .is_none());

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            "myAwesomeMixnode",
            storage::mixnodes()
                .idx
                .owner
                .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
                .unwrap()
                .unwrap()
                .1
                .identity()
        );
    }

    #[test]
    fn adding_mixnode_with_existing_owner() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // request fails giving the existing owner address in the message
        let info = mock_info("mix-owner-pretender", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            Err(ContractError::DuplicateMixnode {
                owner: Addr::unchecked("mix-owner")
            }),
            execute_response
        );
    }

    #[test]
    fn adding_mixnode_with_existing_unchanged_owner() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                host: "1.1.1.1:1789".into(),
                ..test_helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                host: "2.2.2.2:1789".into(),
                ..test_helpers::mix_node_fixture()
            },
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(Err(ContractError::AlreadyOwnsMixnode), res);
    }

    #[test]
    fn adding_mixnode_updates_layer_distribution() {
        let mut deps = test_helpers::init_contract();

        assert_eq!(
            LayerDistribution::default(),
            mixnet_params_storage::LAYERS.load(&deps.storage).unwrap(),
        );

        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "mix1".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            LayerDistribution {
                layer1: 1,
                ..Default::default()
            },
            mixnet_params_storage::LAYERS.load(&deps.storage).unwrap()
        );
    }

    #[test]
    fn mixnode_remove() {
        let mut deps = test_helpers::init_contract();

        // try un-registering when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked("anyone")
            })
        );

        // let's add a node owned by bob
        test_helpers::add_mixnode("bob", test_helpers::good_mixnode_bond(), deps.as_mut());

        // attempt to un-register fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::NoAssociatedMixNodeBond {
                owner: Addr::unchecked("fred")
            })
        );

        // bob's node is still there
        let nodes = test_helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, nodes.len());
        assert_eq!("bob", nodes[0].owner().clone());

        // add a node owned by fred
        let info = mock_info("fred", &test_helpers::good_mixnode_bond());
        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            info,
            MixNode {
                identity_key: "fredsmixnode".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        )
        .unwrap();

        // let's make sure we now have 2 nodes:
        assert_eq!(2, test_helpers::get_mix_nodes(&mut deps).len());

        // un-register fred's node
        let info = mock_info("fred", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        let remove_fred = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr(
                "mixnode_bond",
                format!(
                    "amount: {}{}, owner: fred, identity: fredsmixnode",
                    INITIAL_MIXNODE_BOND, DENOM
                ),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_message = BankMsg::Send {
            to_address: String::from(info.sender),
            amount: test_helpers::good_mixnode_bond(),
        };

        // run the executor and check that we got back the correct results
        let expected = Response::new()
            .add_attributes(expected_attributes)
            .add_message(expected_message);

        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let mix_node_bonds = test_helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, mix_node_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), mix_node_bonds[0].owner());
    }

    #[test]
    fn removing_mixnode_clears_ownership() {
        let mut deps = test_helpers::init_contract();

        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            "myAwesomeMixnode",
            storage::mixnodes()
                .idx
                .owner
                .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
                .unwrap()
                .unwrap()
                .1
                .identity()
        );

        let info = mock_info("mix-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(storage::mixnodes()
            .idx
            .owner
            .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("mix-owner", &test_helpers::good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..test_helpers::mix_node_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());
        assert_eq!(
            "myAwesomeMixnode",
            storage::mixnodes()
                .idx
                .owner
                .item(deps.as_ref().storage, Addr::unchecked("mix-owner"))
                .unwrap()
                .unwrap()
                .1
                .identity()
        );
    }

    #[test]
    fn validating_mixnode_bond() {
        // you must send SOME funds
        let result = validate_mixnode_bond(Vec::new(), INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = test_helpers::good_mixnode_bond();
        bond[0].amount = INITIAL_MIXNODE_BOND.checked_sub(Uint128::new(1)).unwrap();
        let result = validate_mixnode_bond(bond.clone(), INITIAL_MIXNODE_BOND);
        assert_eq!(
            result,
            Err(ContractError::InsufficientMixNodeBond {
                received: Into::<u128>::into(INITIAL_MIXNODE_BOND) - 1,
                minimum: INITIAL_MIXNODE_BOND.into(),
            })
        );

        // more than that is still fine
        let mut bond = test_helpers::good_mixnode_bond();
        bond[0].amount = INITIAL_MIXNODE_BOND + Uint128::new(1);
        let result = validate_mixnode_bond(bond.clone(), INITIAL_MIXNODE_BOND);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = test_helpers::good_mixnode_bond();
        bond[0].denom = "baddenom".to_string();
        let result = validate_mixnode_bond(bond.clone(), INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = test_helpers::good_mixnode_bond();
        bond[0].denom = "foomp".to_string();
        let result = validate_mixnode_bond(bond.clone(), INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }

    #[test]
    fn choose_layer_mix_node() {
        let mut deps = test_helpers::init_contract();
        for owner in ["alice", "bob"] {
            try_add_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &test_helpers::good_mixnode_bond()),
                MixNode {
                    identity_key: owner.to_string(),
                    ..test_helpers::mix_node_fixture()
                },
            )
            .unwrap();
        }
        let bonded_mix_nodes = test_helpers::get_mix_nodes(&mut deps);
        let alice_node = bonded_mix_nodes.get(0).unwrap().clone();
        let bob_node = bonded_mix_nodes.get(1).unwrap().clone();
        assert_eq!(alice_node.mix_node.identity_key, "alice");
        assert_eq!(alice_node.layer, Layer::One);
        assert_eq!(bob_node.mix_node.identity_key, "bob");
        assert_eq!(bob_node.layer, mixnet_contract::Layer::Two);
    }
}
