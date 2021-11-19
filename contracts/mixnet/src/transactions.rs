// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::error::ContractError;
use crate::storage::*;
use config::defaults::DENOM;
use cosmwasm_std::{coins, BankMsg, Coin, DepsMut, Env, MessageInfo, Response};

use mixnet_contract::{IdentityKey, Layer, RawDelegationData};

pub(crate) const OLD_DELEGATIONS_CHUNK_SIZE: usize = 500;

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::bonding_gateways::transactions::try_add_gateway;
    use crate::bonding_gateways::transactions::validate_gateway_bond;
    use crate::bonding_mixnodes::transactions::try_add_mixnode;
    use crate::bonding_mixnodes::transactions::validate_mixnode_bond;
    use crate::contract::{
        execute, query, DEFAULT_SYBIL_RESISTANCE_PERCENT, INITIAL_DEFAULT_EPOCH_LENGTH,
        INITIAL_GATEWAY_BOND, INITIAL_MIXNODE_BOND, INITIAL_MIXNODE_BOND_REWARD_RATE,
        INITIAL_MIXNODE_DELEGATION_REWARD_RATE,
    };
    use crate::delegating_mixnodes::transactions::try_delegate_to_mixnode;
    use crate::helpers::calculate_epoch_reward_rate;
    use crate::helpers::scale_reward_by_uptime;
    use crate::helpers::Delegations;
    use crate::mixnet_params::transactions::try_update_state_params;
    use crate::queries::tests::store_n_mix_delegations;
    use crate::queries::DELEGATION_PAGE_DEFAULT_LIMIT;
    use crate::rewards::transactions::{
        try_begin_mixnode_rewarding, try_finish_mixnode_rewarding, try_reward_mixnode,
        try_reward_mixnode_v2,
    };
    use crate::storage::{layer_distribution_read, mix_delegations_read, read_mixnode_bond};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        add_mixnode, good_gateway_bond, good_mixnode_bond, mix_node_fixture, raw_delegation_fixture,
    };
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Decimal;
    use cosmwasm_std::{coin, coins, from_binary, Addr, Uint128};
    use mixnet_contract::mixnode::NodeRewardParams;
    use mixnet_contract::Gateway;
    use mixnet_contract::MixNode;
    use mixnet_contract::MixNodeBond;
    use mixnet_contract::StateParams;
    use mixnet_contract::{
        ExecuteMsg, LayerDistribution, PagedGatewayResponse, PagedMixnodeResponse, QueryMsg,
        UnpackedDelegation,
    };

    #[test]
    fn validating_mixnode_bond() {
        // you must send SOME funds
        let result = validate_mixnode_bond(&[], INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = good_mixnode_bond();
        bond[0].amount = INITIAL_MIXNODE_BOND.checked_sub(Uint128(1)).unwrap();
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert_eq!(
            result,
            Err(ContractError::InsufficientMixNodeBond {
                received: Into::<u128>::into(INITIAL_MIXNODE_BOND) - 1,
                minimum: INITIAL_MIXNODE_BOND.into(),
            })
        );

        // more than that is still fine
        let mut bond = good_mixnode_bond();
        bond[0].amount = INITIAL_MIXNODE_BOND + Uint128(1);
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = good_mixnode_bond();
        bond[0].denom = "baddenom".to_string();
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = good_mixnode_bond();
        bond[0].denom = "foomp".to_string();
        let result = validate_mixnode_bond(&bond, INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }

    #[test]
    fn mixnode_add() {
        let mut deps = helpers::init_contract();

        // if we don't send enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_MIXNODE_BOND) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "anyonesmixnode".into(),
                ..helpers::mix_node_fixture()
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
        let info = mock_info("anyone", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "anyonesmixnode".into(),
                ..helpers::mix_node_fixture()
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
                ..helpers::mix_node_fixture()
            },
            page.nodes[0].mix_node()
        );

        // if there was already a mixnode bonded by particular user
        let info = mock_info("foomper", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "foompermixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "foompermixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };

        // we get a log message about it (TODO: does it get back to the user?)
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", true));

        // bonding fails if the user already owns a gateway
        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "ownersgateway".into(),
                ..helpers::gateway_fixture()
            },
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsGateway));

        // but after he unbonds it, it's all fine again
        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmixnode".into(),
                ..helpers::mix_node_fixture()
            },
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state). Is that right? Think about this, not sure yet.
        // if we attempt to register a second node from the same address, should we get an error? It would probably be polite.
    }

    #[test]
    fn adding_mixnode_without_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        // before the execution the node had no associated owner
        assert!(mixnodes_owners_read(deps.as_ref().storage)
            .may_load("myAwesomeMixnode".as_bytes())
            .unwrap()
            .is_none());

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            "myAwesomeMixnode",
            mixnodes_owners_read(deps.as_ref().storage)
                .load("mix-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn adding_mixnode_with_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // request fails giving the existing owner address in the message
        let info = mock_info("mix-owner-pretender", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
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
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                host: "1.1.1.1:1789".into(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                host: "2.2.2.2:1789".into(),
                ..helpers::mix_node_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        // make sure the host information was updated
        assert_eq!(
            "2.2.2.2:1789".to_string(),
            mixnodes_read(deps.as_ref().storage)
                .load("myAwesomeMixnode".as_bytes())
                .unwrap()
                .mix_node
                .host
        );
    }

    #[test]
    fn adding_mixnode_updates_layer_distribution() {
        let mut deps = helpers::init_contract();

        assert_eq!(
            LayerDistribution::default(),
            layer_distribution_read(&deps.storage).load().unwrap(),
        );

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "mix1".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            LayerDistribution {
                layer1: 1,
                ..Default::default()
            },
            layer_distribution_read(&deps.storage).load().unwrap()
        );
    }

    #[test]
    fn mixnode_remove() {
        let mut deps = helpers::init_contract();

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
        helpers::add_mixnode("bob", good_mixnode_bond(), &mut deps);

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
        let nodes = helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, nodes.len());
        assert_eq!("bob", nodes[0].owner().clone());

        // add a node owned by fred
        let info = mock_info("fred", &good_mixnode_bond());
        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            info,
            MixNode {
                identity_key: "fredsmixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_mix_nodes(&mut deps).len());

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
                    "amount: {} {}, owner: fred, identity: fredsmixnode",
                    INITIAL_MIXNODE_BOND, DENOM
                ),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_messages = vec![BankMsg::Send {
            to_address: String::from(info.sender),
            amount: good_mixnode_bond(),
        }
        .into()];

        // run the executer and check that we got back the correct results
        let expected = Response {
            submessages: Vec::new(),
            messages: expected_messages,
            attributes: expected_attributes,
            data: None,
        };
        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let mix_node_bonds = helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, mix_node_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), mix_node_bonds[0].owner());
    }

    #[test]
    fn removing_mixnode_clears_ownership() {
        let mut deps = helpers::init_contract();

        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            "myAwesomeMixnode",
            mixnodes_owners_read(deps.as_ref().storage)
                .load("mix-owner".as_bytes())
                .unwrap()
        );

        let info = mock_info("mix-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(mixnodes_owners_read(deps.as_ref().storage)
            .may_load("mix-owner".as_bytes())
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("mix-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "myAwesomeMixnode".to_string(),
                ..helpers::mix_node_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());
        assert_eq!(
            "myAwesomeMixnode",
            mixnodes_owners_read(deps.as_ref().storage)
                .load("mix-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn validating_gateway_bond() {
        // you must send SOME funds
        let result = validate_gateway_bond(&[], INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = good_gateway_bond();
        bond[0].amount = INITIAL_GATEWAY_BOND.checked_sub(Uint128(1)).unwrap();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: Into::<u128>::into(INITIAL_GATEWAY_BOND) - 1,
                minimum: INITIAL_GATEWAY_BOND.into(),
            })
        );

        // more than that is still fine
        let mut bond = good_gateway_bond();
        bond[0].amount = INITIAL_GATEWAY_BOND + Uint128(1);
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert!(result.is_ok());

        // it must be sent in the defined denom!
        let mut bond = good_gateway_bond();
        bond[0].denom = "baddenom".to_string();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut bond = good_gateway_bond();
        bond[0].denom = "foomp".to_string();
        let result = validate_gateway_bond(&bond, INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }

    #[test]
    fn gateway_add() {
        let mut deps = helpers::init_contract();

        // if we fail validation (by say not sending enough funds
        let insufficient_bond = Into::<u128>::into(INITIAL_GATEWAY_BOND) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, DENOM));
        let msg = ExecuteMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
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
        let info = mock_info("anyone", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "anyonesgateway".into(),
                ..helpers::gateway_fixture()
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
                ..helpers::gateway_fixture()
            },
            page.nodes[0].gateway()
        );

        // if there was already a gateway bonded by particular user
        let info = mock_info("foomper", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "foompersgateway".into(),
                ..helpers::gateway_fixture()
            },
        };

        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "foompersgateway".into(),
                ..helpers::gateway_fixture()
            },
        };

        // we get a log message about it (TODO: does it get back to the user?)
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(execute_response.attributes[0], attr("overwritten", true));

        // bonding fails if the user already owns a mixnode
        let info = mock_info("mixnode-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondMixnode {
            mix_node: MixNode {
                identity_key: "ownersmix".into(),
                ..helpers::mix_node_fixture()
            },
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(execute_response, Err(ContractError::AlreadyOwnsMixnode));

        // but after he unbonds it, it's all fine again
        let info = mock_info("mixnode-owner", &[]);
        let msg = ExecuteMsg::UnbondMixnode {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("mixnode-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state).
        // Is that right? Think about this, not sure yet.
    }

    #[test]
    fn adding_gateway_without_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        // before the execution the node had no associated owner
        assert!(gateways_owners_read(deps.as_ref().storage)
            .may_load("gateway-owner".as_bytes())
            .unwrap()
            .is_none());

        // it's all fine, owner is saved
        let execute_response = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(execute_response.is_ok());

        assert_eq!(
            "myAwesomeGateway",
            gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn adding_gateway_with_existing_owner() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // request fails giving the existing owner address in the message
        let info = mock_info("gateway-owner-pretender", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
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
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                host: "1.1.1.1".into(),
                ..helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("gateway-owner", &good_gateway_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                host: "2.2.2.2".into(),
                ..helpers::gateway_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        // make sure the host information was updated
        assert_eq!(
            "2.2.2.2".to_string(),
            gateways_read(deps.as_ref().storage)
                .load("myAwesomeGateway".as_bytes())
                .unwrap()
                .gateway
                .host
        );
    }

    #[test]
    fn gateway_remove() {
        let mut deps = helpers::init_contract();

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
        helpers::add_gateway("bob", good_gateway_bond(), &mut deps);

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
        let nodes = helpers::get_gateways(&mut deps);
        assert_eq!(1, nodes.len());

        let first_node = &nodes[0];
        assert_eq!(&Addr::unchecked("bob"), first_node.owner());

        // add a node owned by fred
        let info = mock_info("fred", &good_gateway_bond());
        try_add_gateway(
            deps.as_mut(),
            mock_env(),
            info,
            Gateway {
                identity_key: "fredsgateway".into(),
                ..helpers::gateway_fixture()
            },
        )
        .unwrap();

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_gateways(&mut deps).len());

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
        let expected_messages = vec![BankMsg::Send {
            to_address: String::from(info.sender),
            amount: good_gateway_bond(),
        }
        .into()];

        // run the executer and check that we got back the correct results
        let expected = Response {
            submessages: Vec::new(),
            messages: expected_messages,
            attributes: expected_attributes,
            data: None,
        };
        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let gateway_bonds = helpers::get_gateways(&mut deps);
        assert_eq!(1, gateway_bonds.len());
        assert_eq!(&Addr::unchecked("bob"), gateway_bonds[0].owner());
    }

    #[test]
    fn removing_gateway_clears_ownership() {
        let mut deps = helpers::init_contract();

        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            "myAwesomeGateway",
            gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );

        let info = mock_info("gateway-owner", &[]);
        let msg = ExecuteMsg::UnbondGateway {};

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());

        assert!(gateways_owners_read(deps.as_ref().storage)
            .may_load("gateway-owner".as_bytes())
            .unwrap()
            .is_none());

        // and since it's removed, it can be reclaimed
        let info = mock_info("gateway-owner", &good_mixnode_bond());
        let msg = ExecuteMsg::BondGateway {
            gateway: Gateway {
                identity_key: "myAwesomeGateway".to_string(),
                ..helpers::gateway_fixture()
            },
        };

        assert!(execute(deps.as_mut(), mock_env(), info, msg).is_ok());
        assert_eq!(
            "myAwesomeGateway",
            gateways_owners_read(deps.as_ref().storage)
                .load("gateway-owner".as_bytes())
                .unwrap()
        );
    }

    #[test]
    fn updating_state_params() {
        let mut deps = helpers::init_contract();

        let new_params = StateParams {
            epoch_length: INITIAL_DEFAULT_EPOCH_LENGTH,
            minimum_mixnode_bond: INITIAL_MIXNODE_BOND,
            minimum_gateway_bond: INITIAL_GATEWAY_BOND,
            mixnode_bond_reward_rate: Decimal::percent(INITIAL_MIXNODE_BOND_REWARD_RATE),
            mixnode_delegation_reward_rate: Decimal::percent(
                INITIAL_MIXNODE_DELEGATION_REWARD_RATE,
            ),
            mixnode_rewarded_set_size: 100,
            mixnode_active_set_size: 50,
        };

        // cannot be updated from non-owner account
        let info = mock_info("not-the-creator", &[]);
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Err(ContractError::Unauthorized));

        // but works fine from the creator account
        let info = mock_info("creator", &[]);
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Ok(Response::default()));

        // and the state is actually updated
        let current_state = config_read(deps.as_ref().storage).load().unwrap();
        assert_eq!(current_state.params, new_params);

        // mixnode_epoch_rewards are recalculated if annual reward  is changed
        let current_mix_bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let current_mix_delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let new_mixnode_bond_reward_rate = Decimal::percent(120);
        let new_mixnode_delegation_reward_rate = Decimal::percent(120);

        // sanity check to make sure we are actually updating the values (in case we changed defaults at some point)
        assert_ne!(new_mixnode_bond_reward_rate, current_mix_bond_reward_rate);
        assert_ne!(
            new_mixnode_delegation_reward_rate,
            current_mix_delegation_reward_rate
        );

        let mut new_params = current_state.params.clone();
        new_params.mixnode_bond_reward_rate = new_mixnode_bond_reward_rate;
        new_params.mixnode_delegation_reward_rate = new_mixnode_delegation_reward_rate;

        let info = mock_info("creator", &[]);
        try_update_state_params(deps.as_mut(), info, new_params.clone()).unwrap();

        let new_state = config_read(deps.as_ref().storage).load().unwrap();
        let expected_bond =
            calculate_epoch_reward_rate(new_params.epoch_length, new_mixnode_bond_reward_rate);
        let expected_delegation = calculate_epoch_reward_rate(
            new_params.epoch_length,
            new_mixnode_delegation_reward_rate,
        );
        assert_eq!(expected_bond, new_state.mixnode_epoch_bond_reward);
        assert_eq!(
            expected_delegation,
            new_state.mixnode_epoch_delegation_reward
        );

        // mixnode_epoch_rewards is updated on epoch length change
        let new_epoch_length = 42;
        // sanity check to make sure we are actually updating the value (in case we changed defaults at some point)
        assert_ne!(new_epoch_length, current_state.params.epoch_length);
        let mut new_params = current_state.params.clone();
        new_params.epoch_length = new_epoch_length;

        let info = mock_info("creator", &[]);
        try_update_state_params(deps.as_mut(), info, new_params.clone()).unwrap();

        let new_state = config_read(deps.as_ref().storage).load().unwrap();
        let expected_mixnode_bond =
            calculate_epoch_reward_rate(new_epoch_length, new_params.mixnode_bond_reward_rate);
        let expected_mixnode_delegation = calculate_epoch_reward_rate(
            new_epoch_length,
            new_params.mixnode_delegation_reward_rate,
        );
        assert_eq!(expected_mixnode_bond, new_state.mixnode_epoch_bond_reward);
        assert_eq!(
            expected_mixnode_delegation,
            new_state.mixnode_epoch_delegation_reward
        );

        // error is thrown if rewarded set is smaller than the active set
        let info = mock_info("creator", &[]);
        let mut new_params = current_state.params.clone();
        new_params.mixnode_rewarded_set_size = new_params.mixnode_active_set_size - 1;
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(Err(ContractError::InvalidActiveSetSize), res)
    }

    #[cfg(test)]
    mod beginning_mixnode_rewarding {
        use super::*;
        use crate::rewards::transactions::try_begin_mixnode_rewarding;

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info("not-the-approved-validator", &[]),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_already_in_progress_with_little_day() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            );
            assert_eq!(Err(ContractError::RewardingInProgress), res);
        }

        #[test]
        fn can_be_called_if_rewarding_is_in_progress_if_sufficient_number_of_blocks_elapsed() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let mut new_env = env.clone();

            new_env.block.height = env.block.height + MAX_REWARDING_DURATION_IN_BLOCKS;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                new_env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                2,
            );
            assert!(res.is_ok());
        }

        #[test]
        fn provided_nonce_must_be_equal_the_current_plus_one() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let mut current_state = config_read(deps.as_mut().storage).load().unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            config(deps.as_mut().storage).save(&current_state).unwrap();

            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                11,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 11,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                44,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 44,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                42,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 42,
                    expected: 43
                }),
                res
            );

            let res = try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn updates_contract_state() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let start_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = start_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env.clone(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = config_read(deps.as_mut().storage).load().unwrap();
            assert!(new_state.rewarding_in_progress);
            assert_eq!(
                new_state.rewarding_interval_starting_block,
                env.block.height
            );
            assert_eq!(
                start_state.latest_rewarding_interval_nonce + 1,
                new_state.latest_rewarding_interval_nonce
            );
        }
    }

    #[cfg(test)]
    mod finishing_mixnode_rewarding {
        use super::*;
        use crate::rewards::transactions::{
            try_begin_mixnode_rewarding, try_finish_mixnode_rewarding,
        };

        #[test]
        fn can_only_be_called_by_specified_validator_address() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info("not-the-approved-validator", &[]),
                1,
            );
            assert_eq!(Err(ContractError::Unauthorized), res);

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn cannot_be_called_if_rewarding_is_not_in_progress() {
            let mut deps = helpers::init_contract();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                0,
            );
            assert_eq!(Err(ContractError::RewardingNotInProgress), res);
        }

        #[test]
        fn provided_nonce_must_be_equal_the_current_one() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let mut current_state = config_read(deps.as_mut().storage).load().unwrap();
            current_state.latest_rewarding_interval_nonce = 42;
            config(deps.as_mut().storage).save(&current_state).unwrap();

            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            )
            .unwrap();

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                11,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 11,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                44,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 44,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                42,
            );
            assert_eq!(
                Err(ContractError::InvalidRewardingIntervalNonce {
                    received: 42,
                    expected: 43
                }),
                res
            );

            let res = try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                43,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn updates_contract_state() {
            let mut deps = helpers::init_contract();
            let env = mock_env();
            let current_state = config_read(deps.as_mut().storage).load().unwrap();
            let rewarding_validator_address = current_state.rewarding_validator_address;

            try_begin_mixnode_rewarding(
                deps.as_mut(),
                env,
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            try_finish_mixnode_rewarding(
                deps.as_mut(),
                mock_info(rewarding_validator_address.as_ref(), &[]),
                1,
            )
            .unwrap();

            let new_state = config_read(deps.as_mut().storage).load().unwrap();
            assert!(!new_state.rewarding_in_progress);
        }
    }

    #[test]
    fn rewarding_mixnode() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // errors out if executed by somebody else than network monitor
        let info = mock_info("not-the-monitor", &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        );
        assert_eq!(res, Err(ContractError::Unauthorized));

        // begin rewarding period
        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        // returns bond not found attribute if the target owner hasn't bonded any mixnodes
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        )
        .unwrap();
        assert_eq!(vec![attr("result", "bond not found")], res.attributes);

        let initial_bond = 100_000000;
        let initial_delegation = 200_000000;
        let mixnode_bond = MixNodeBond {
            bond_amount: coin(initial_bond, DENOM),
            total_delegation: coin(initial_delegation, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        mix_delegations(&mut deps.storage, &node_identity)
            .save(
                b"delegator",
                &RawDelegationData::new(initial_delegation.into(), env.block.height),
            )
            .unwrap();

        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let expected_bond_reward = Uint128(initial_bond) * bond_reward_rate;
        let expected_delegation_reward = Uint128(initial_delegation) * delegation_reward_rate;

        // the node's bond and delegations are correctly increased and scaled by uptime
        // if node was 100% up, it will get full epoch reward
        let expected_bond = expected_bond_reward + Uint128(initial_bond);
        let expected_delegation = expected_delegation_reward + Uint128(initial_delegation);

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_bond_reward = scale_reward_by_uptime(bond_reward_rate, 20).unwrap();
        let scaled_delegation_reward = scale_reward_by_uptime(delegation_reward_rate, 20).unwrap();
        let expected_bond_reward = expected_bond * scaled_bond_reward;
        let expected_delegation_reward = expected_delegation * scaled_delegation_reward;
        let expected_bond = expected_bond_reward + expected_bond;
        let expected_delegation = expected_delegation_reward + expected_delegation;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);

        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            20,
            2,
        )
        .unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );
    }

    #[test]
    fn rewarding_mixnodes_outside_rewarding_period() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        );
        assert_eq!(Err(ContractError::RewardingNotInProgress), res);

        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn rewarding_mixnodes_with_incorrect_rewarding_nonce() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            0,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 0,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            2,
        );
        assert_eq!(
            Err(ContractError::InvalidRewardingIntervalNonce {
                received: 2,
                expected: 1
            }),
            res
        );

        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info,
            node_identity.clone(),
            100,
            1,
        );
        assert!(res.is_ok())
    }

    #[test]
    fn attempting_rewarding_mixnode_multiple_times_per_interval() {
        let mut deps = helpers::init_contract();
        let env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        // bond the node
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        try_add_mixnode(
            deps.as_mut(),
            env.clone(),
            mock_info(node_owner.as_ref(), &good_mixnode_bond()),
            MixNode {
                identity_key: node_identity.to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();

        // first reward goes through just fine
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        );
        assert!(res.is_ok());

        // but the other one fails
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        );
        assert_eq!(
            Err(ContractError::MixnodeAlreadyRewarded {
                identity: node_identity.clone()
            }),
            res
        );

        // but rewarding the same node in the following interval is fine again
        try_finish_mixnode_rewarding(deps.as_mut(), info.clone(), 1).unwrap();
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();

        let res = try_reward_mixnode(deps.as_mut(), env, info, node_identity.clone(), 100, 2);
        assert!(res.is_ok());
    }

    #[test]
    fn rewarding_mixnode_blockstamp_based() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config_read(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;

        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        let initial_bond = 100_000000;
        let initial_delegation = 200_000000;
        let mixnode_bond = MixNodeBond {
            bond_amount: coin(initial_bond, DENOM),
            total_delegation: coin(initial_delegation, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: env.block.height,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        mixnodes(deps.as_mut().storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        // delegation happens later, but not later enough
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;

        mix_delegations(&mut deps.storage, &node_identity)
            .save(
                b"delegator",
                &RawDelegationData::new(initial_delegation.into(), env.block.height),
            )
            .unwrap();

        let bond_reward_rate = current_state.mixnode_epoch_bond_reward;
        let delegation_reward_rate = current_state.mixnode_epoch_delegation_reward;
        let scaled_bond_reward = scale_reward_by_uptime(bond_reward_rate, 100).unwrap();
        let scaled_delegation_reward = scale_reward_by_uptime(delegation_reward_rate, 100).unwrap();

        // no reward is due
        let expected_bond_reward = Uint128(0);
        let expected_delegation_reward = Uint128(0);
        let expected_bond = expected_bond_reward + Uint128(initial_bond);
        let expected_delegation = expected_delegation_reward + Uint128(initial_delegation);

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 1).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            1,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 1).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // reward can happen now, but only for bonded node
        env.block.height += 1;
        let expected_bond_reward = expected_bond * scaled_bond_reward;
        let expected_delegation_reward = Uint128(0);
        let expected_bond = expected_bond_reward + expected_bond;
        let expected_delegation = expected_delegation_reward + expected_delegation;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 2).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            2,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 2).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );

        // reward happens now, both for node owner and delegators
        env.block.height += MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;
        let expected_bond_reward = expected_bond * scaled_bond_reward;
        let expected_delegation_reward = expected_delegation * scaled_delegation_reward;
        let expected_bond = expected_bond_reward + expected_bond;
        let expected_delegation = expected_delegation_reward + expected_delegation;

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(deps.as_mut(), env.clone(), info.clone(), 3).unwrap();
        let res = try_reward_mixnode(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            node_identity.clone(),
            100,
            3,
        )
        .unwrap();
        try_finish_mixnode_rewarding(deps.as_mut(), info, 3).unwrap();

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );
        assert_eq!(
            expected_delegation,
            read_mixnode_delegation(deps.as_ref().storage, node_identity.as_bytes()).unwrap()
        );

        assert_eq!(
            vec![
                attr("bond increase", expected_bond_reward),
                attr("total delegation increase", expected_delegation_reward),
            ],
            res.attributes
        );
    }

    #[test]
    fn choose_layer_mix_node() {
        let mut deps = helpers::init_contract();
        for owner in ["alice", "bob"] {
            try_add_mixnode(
                deps.as_mut(),
                mock_env(),
                mock_info(owner, &good_mixnode_bond()),
                MixNode {
                    identity_key: owner.to_string(),
                    ..helpers::mix_node_fixture()
                },
            )
            .unwrap();
        }
        let bonded_mix_nodes = helpers::get_mix_nodes(&mut deps);
        let alice_node = bonded_mix_nodes.get(0).unwrap().clone();
        let bob_node = bonded_mix_nodes.get(1).unwrap().clone();
        assert_eq!(alice_node.mix_node.identity_key, "alice");
        assert_eq!(alice_node.layer, Layer::One);
        assert_eq!(bob_node.mix_node.identity_key, "bob");
        assert_eq!(bob_node.layer, mixnet_contract::Layer::Two);
    }

    #[test]
    fn test_tokenomics_rewarding() {
        use crate::contract::{EPOCH_REWARD_PERCENT, INITIAL_REWARD_POOL};

        type U128 = fixed::types::U75F53;

        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        let current_state = config(deps.as_mut().storage).load().unwrap();
        let rewarding_validator_address = current_state.rewarding_validator_address;
        let period_reward_pool = (INITIAL_REWARD_POOL / 100) * EPOCH_REWARD_PERCENT as u128;
        assert_eq!(period_reward_pool, 5_000_000_000_000);
        let k = 200; // Imagining our active set size is 200
        let circulating_supply = circulating_supply(&deps.storage).u128();
        assert_eq!(circulating_supply, 750_000_000_000_000u128);
        // mut_reward_pool(deps.as_mut().storage)
        //     .save(&Uint128(period_reward_pool))
        //     .unwrap();

        try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info(
                "alice",
                &vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128(10_000_000_000),
                }],
            ),
            MixNode {
                identity_key: "alice".to_string(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d1", &vec![coin(8000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        try_delegate_to_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("d2", &vec![coin(2000_000000, DENOM)]),
            "alice".to_string(),
        )
        .unwrap();

        let info = mock_info(rewarding_validator_address.as_ref(), &[]);
        try_begin_mixnode_rewarding(
            deps.as_mut(),
            env.clone(),
            mock_info(rewarding_validator_address.as_ref(), &[]),
            1,
        )
        .unwrap();

        env.block.height += 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING;

        let mix_1 = mixnodes_read(&deps.storage).load(b"alice").unwrap();
        let mix_1_uptime = 100;

        let mut params = NodeRewardParams::new(
            period_reward_pool,
            k,
            0,
            circulating_supply,
            mix_1_uptime,
            DEFAULT_SYBIL_RESISTANCE_PERCENT,
        );

        params.set_reward_blockstamp(env.block.height);

        assert_eq!(params.performance(), 1);

        let mix_1_reward_result = mix_1.reward(&params);

        assert_eq!(
            mix_1_reward_result.sigma(),
            U128::from_num(0.0000266666666666)
        );
        assert_eq!(
            mix_1_reward_result.lambda(),
            U128::from_num(0.0000133333333333)
        );
        assert_eq!(mix_1_reward_result.reward().int(), 102646153);

        let mix1_operator_profit = mix_1.operator_reward(&params);

        let mix1_delegator1_reward = mix_1.reward_delegation(Uint128(8000_000000), &params);

        let mix1_delegator2_reward = mix_1.reward_delegation(Uint128(2000_000000), &params);

        assert_eq!(mix1_operator_profit, U128::from_num(74455384));
        assert_eq!(mix1_delegator1_reward, U128::from_num(22552615));
        assert_eq!(mix1_delegator2_reward, U128::from_num(5638153));

        let pre_reward_bond = read_mixnode_bond(&deps.storage, b"alice").unwrap().u128();
        assert_eq!(pre_reward_bond, 10_000_000_000);

        let pre_reward_delegation = read_mixnode_delegation(&deps.storage, b"alice")
            .unwrap()
            .u128();
        assert_eq!(pre_reward_delegation, 10_000_000_000);

        try_reward_mixnode_v2(deps.as_mut(), env, info, "alice".to_string(), params, 1).unwrap();

        assert_eq!(
            read_mixnode_bond(&deps.storage, b"alice").unwrap().u128(),
            U128::from_num(pre_reward_bond) + U128::from_num(mix1_operator_profit)
        );
        assert_eq!(
            read_mixnode_delegation(&deps.storage, b"alice")
                .unwrap()
                .u128(),
            pre_reward_delegation + mix1_delegator1_reward + mix1_delegator2_reward
        );

        assert_eq!(
            reward_pool_value(&deps.storage).u128(),
            U128::from_num(INITIAL_REWARD_POOL)
                - (U128::from_num(mix1_operator_profit)
                    + U128::from_num(mix1_delegator1_reward)
                    + U128::from_num(mix1_delegator2_reward))
        )
    }
}
