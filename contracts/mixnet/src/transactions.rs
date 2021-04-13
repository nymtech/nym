use crate::contract::DENOM;
use crate::error::ContractError;
use crate::helpers::scale_reward_by_uptime;
use crate::state::StateParams;
use crate::storage::{
    config, config_read, gateways, gateways_read, increase_gateway_bond, increase_mixnode_bond,
    mixnodes, mixnodes_read, read_gateway_epoch_reward_rate, read_mixnode_epoch_reward_rate,
    read_state_params,
};
use cosmwasm_std::{
    attr, BankMsg, Coin, Decimal, DepsMut, Env, HandleResponse, HumanAddr, MessageInfo, Uint128,
};
use mixnet_contract::{Gateway, GatewayBond, MixNode, MixNodeBond};

fn validate_mixnode_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
    // check if anything was put as bond
    if bond.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if bond.len() > 1 {
        // TODO: ask DH what would be an appropriate action here
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

    Ok(())
}

pub(crate) fn try_add_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    mix_node: MixNode,
) -> Result<HandleResponse, ContractError> {
    let minimum_bond = read_state_params(deps.storage).minimum_mixnode_bond;
    validate_mixnode_bond(&info.sent_funds, minimum_bond)?;

    let bond = MixNodeBond::new(info.sent_funds, info.sender.clone(), mix_node);

    let sender_bytes = info.sender.as_bytes();
    let was_present = mixnodes_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some();

    // TODO: do attributes also go back to the client or does this need to be put into `data`?
    let attributes = vec![attr("overwritten", was_present)];

    mixnodes(deps.storage).save(sender_bytes, &bond)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes,
        data: None,
    })
}

pub(crate) fn try_remove_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<HandleResponse, ContractError> {
    // find the bond, return ContractError::MixNodeBondNotFound if it doesn't exist
    let mixnode_bond = match mixnodes_read(deps.storage).may_load(info.sender.as_bytes())? {
        None => return Err(ContractError::MixNodeBondNotFound {}),
        Some(bond) => bond,
    };

    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        from_address: env.contract.address,
        to_address: info.sender.clone(),
        amount: mixnode_bond.amount().to_vec(),
    }
    .into()];

    // remove the bond from the list of bonded mixnodes
    mixnodes(deps.storage).remove(info.sender.as_bytes());

    // log our actions
    let attributes = vec![attr("action", "unbond"), attr("mixnode_bond", mixnode_bond)];

    Ok(HandleResponse {
        messages,
        attributes,
        data: None,
    })
}

fn validate_gateway_bond(bond: &[Coin], minimum_bond: Uint128) -> Result<(), ContractError> {
    // check if anything was put as bond
    if bond.is_empty() {
        return Err(ContractError::NoBondFound);
    }

    if bond.len() > 1 {
        // TODO: ask DH what would be an appropriate action here
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

pub(crate) fn try_add_gateway(
    deps: DepsMut,
    info: MessageInfo,
    gateway: Gateway,
) -> Result<HandleResponse, ContractError> {
    let minimum_bond = read_state_params(deps.storage).minimum_gateway_bond;
    validate_gateway_bond(&info.sent_funds, minimum_bond)?;

    let bond = GatewayBond::new(info.sent_funds, info.sender.clone(), gateway);

    let sender_bytes = info.sender.as_bytes();
    let was_present = gateways_read(deps.storage)
        .may_load(sender_bytes)?
        .is_some();

    // TODO: do attributes also go back to the client or does this need to be put into `data`?
    let attributes = vec![attr("overwritten", was_present)];

    gateways(deps.storage).save(sender_bytes, &bond)?;
    Ok(HandleResponse {
        messages: vec![],
        attributes,
        data: None,
    })
}

pub(crate) fn try_remove_gateway(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<HandleResponse, ContractError> {
    let sender_bytes = info.sender.as_bytes();

    // find the bond, return ContractError::GatewayBondNotFound if it doesn't exist
    let gateway_bond = match gateways_read(deps.storage).may_load(sender_bytes)? {
        None => {
            return Err(ContractError::GatewayBondNotFound {
                account: info.sender,
            });
        }
        Some(bond) => bond,
    };

    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        from_address: env.contract.address,
        to_address: info.sender.clone(),
        amount: gateway_bond.amount().to_vec(),
    }
    .into()];

    // remove the bond from the list of bonded gateways
    gateways(deps.storage).remove(sender_bytes);

    // log our actions
    let attributes = vec![
        attr("action", "unbond"),
        attr("address", info.sender),
        attr("gateway_bond", gateway_bond),
    ];

    Ok(HandleResponse {
        messages,
        attributes,
        data: None,
    })
}

pub(crate) fn try_update_state_params(
    deps: DepsMut,
    info: MessageInfo,
    params: StateParams,
) -> Result<HandleResponse, ContractError> {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    let mut state = config_read(deps.storage).load().unwrap();

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized);
    }

    if params.mixnode_bond_reward_rate < Decimal::one() {
        return Err(ContractError::DecreasingMixnodeBondReward);
    }

    if params.gateway_bond_reward_rate < Decimal::one() {
        return Err(ContractError::DecreasingGatewayBondReward);
    }

    state.params = params;
    config(deps.storage).save(&state)?;

    Ok(HandleResponse::default())
}

pub(crate) fn try_reward_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    node_owner: HumanAddr,
    uptime: u32,
) -> Result<HandleResponse, ContractError> {
    let mut state = config_read(deps.storage).load().unwrap();

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.network_monitor_address {
        return Err(ContractError::Unauthorized);
    }

    let reward = read_mixnode_epoch_reward_rate(deps.storage);
    let scaled_reward = scale_reward_by_uptime(reward, uptime)?;

    increase_mixnode_bond(deps.storage, node_owner.as_bytes(), scaled_reward)?;

    Ok(HandleResponse::default())
}

pub(crate) fn try_reward_gateway(
    deps: DepsMut,
    info: MessageInfo,
    gateway_owner: HumanAddr,
    uptime: u32,
) -> Result<HandleResponse, ContractError> {
    let mut state = config_read(deps.storage).load().unwrap();

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.network_monitor_address {
        return Err(ContractError::Unauthorized);
    }

    let reward = read_gateway_epoch_reward_rate(deps.storage);
    let scaled_reward = scale_reward_by_uptime(reward, uptime)?;

    increase_gateway_bond(deps.storage, gateway_owner.as_bytes(), scaled_reward)?;

    Ok(HandleResponse::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::contract::{
        handle, init, query, INITIAL_GATEWAY_BOND, INITIAL_MIXNODE_BOND, NETWORK_MONITOR_ADDRESS,
    };
    use crate::msg::{HandleMsg, InitMsg, QueryMsg};
    use crate::state::StateParams;
    use crate::storage::{read_gateway_bond, read_gateway_epoch_reward_rate, read_mixnode_bond};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{gateway_fixture, mix_node_fixture};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Uint128};
    use mixnet_contract::{PagedGatewayResponse, PagedResponse};

    fn good_mixnode_bond() -> Vec<Coin> {
        vec![Coin {
            denom: DENOM.to_string(),
            amount: INITIAL_MIXNODE_BOND,
        }]
    }

    #[test]
    fn validating_mixnode_bond() {
        // you must send SOME funds
        let result = validate_mixnode_bond(&[], INITIAL_MIXNODE_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = good_mixnode_bond();
        bond[0].amount = (INITIAL_MIXNODE_BOND - Uint128(1)).unwrap();
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
        let msg = HandleMsg::BondMixnode {
            mix_node: helpers::mix_node_fixture(),
        };

        // we are informed that we didn't send enough funds
        let result = handle(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientMixNodeBond {
                received: insufficient_bond,
                minimum: INITIAL_GATEWAY_BOND.into(),
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
        let page: PagedResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len());

        // if we send enough funds
        let info = mock_info("anyone", &coins(1000_000000, DENOM));
        let msg = HandleMsg::BondMixnode {
            mix_node: helpers::mix_node_fixture(),
        };

        // we get back a message telling us everything was OK
        let handle_response = handle(deps.as_mut(), mock_env(), info, msg);
        assert!(handle_response.is_ok());

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
        let page: PagedResponse = from_binary(&query_response).unwrap();
        assert_eq!(1, page.nodes.len());
        assert_eq!(&helpers::mix_node_fixture(), page.nodes[0].mix_node());

        // if there was already a mixnode bonded by particular user
        let info = mock_info("foomper", &good_mixnode_bond());
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(handle_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_mixnode_bond());
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        // we get a log message about it (TODO: does it get back to the user?)
        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(handle_response.attributes[0], attr("overwritten", true));

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state). Is that right? Think about this, not sure yet.
        // if we attempt to register a second node from the same address, should we get an error? It would probably be polite.
    }

    #[test]
    fn mixnode_remove() {
        let env = mock_env();
        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {};
        let info = mock_info("creator", &[]);
        init(deps.as_mut(), env.clone(), info, msg).unwrap();

        // try un-registering when no nodes exist yet
        let info = mock_info("anyone", &coins(999_9999, DENOM));
        let msg = HandleMsg::UnbondMixnode {};
        let result = handle(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(result, Err(ContractError::MixNodeBondNotFound {}));

        // let's add a node owned by bob
        helpers::add_mixnode("bob", coins(1000_000000, DENOM), &mut deps);

        // attempt to un-register fred's node, which doesn't exist
        let info = mock_info("fred", &coins(999_9999, DENOM));
        let msg = HandleMsg::UnbondMixnode {};
        let result = handle(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(result, Err(ContractError::MixNodeBondNotFound {}));

        // bob's node is still there
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedResponse = from_binary(&res).unwrap();
        let first_node = &page.nodes[0];
        assert_eq!(1, page.nodes.len());
        assert_eq!("bob", first_node.owner());

        // add a node owned by fred
        let fred_bond = good_mixnode_bond();
        helpers::add_mixnode("fred", fred_bond.clone(), &mut deps);

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_mix_nodes(&mut deps).len());

        // un-register fred's node
        let info = mock_info("fred", &coins(999_9999, DENOM));
        let msg = HandleMsg::UnbondMixnode {};
        let remove_fred = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr(
                "mixnode_bond",
                format!("amount: {} {}, owner: fred", INITIAL_MIXNODE_BOND, DENOM),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_messages = vec![BankMsg::Send {
            from_address: env.contract.address,
            to_address: info.sender,
            amount: fred_bond,
        }
        .into()];

        // run the handler and check that we got back the correct results
        let expected = HandleResponse {
            messages: expected_messages,
            attributes: expected_attributes,
            data: None,
        };
        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let mix_node_bonds = helpers::get_mix_nodes(&mut deps);
        assert_eq!(1, mix_node_bonds.len());
        assert_eq!("bob", mix_node_bonds[0].owner());
    }

    fn good_gateway_bond() -> Vec<Coin> {
        vec![Coin {
            denom: DENOM.to_string(),
            amount: INITIAL_GATEWAY_BOND,
        }]
    }

    #[test]
    fn validating_gateway_bond() {
        // you must send SOME funds
        let result = validate_gateway_bond(&[], INITIAL_GATEWAY_BOND);
        assert_eq!(result, Err(ContractError::NoBondFound));

        // you must send at least 100 coins...
        let mut bond = good_gateway_bond();
        bond[0].amount = (INITIAL_GATEWAY_BOND - Uint128(1)).unwrap();
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
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        // we are informed that we didn't send enough funds
        let result = handle(deps.as_mut(), mock_env(), info, msg);
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
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        // we get back a message telling us everything was OK
        let handle_response = handle(deps.as_mut(), mock_env(), info, msg);
        assert!(handle_response.is_ok());

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
        assert_eq!(&helpers::gateway_fixture(), page.nodes[0].gateway());

        // if there was already a gateway bonded by particular user
        let info = mock_info("foomper", &good_gateway_bond());
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(handle_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_gateway_bond());
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        // we get a log message about it (TODO: does it get back to the user?)
        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(handle_response.attributes[0], attr("overwritten", true));

        // adding another node from another account, but with the same IP, should fail (or we would have a weird state).
        // Is that right? Think about this, not sure yet.
    }

    #[test]
    fn gateway_remove() {
        let env = mock_env();
        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {};
        let info = mock_info("creator", &[]);
        init(deps.as_mut(), env.clone(), info, msg).unwrap();

        // try unbond when no nodes exist yet
        let info = mock_info("anyone", &[]);
        let msg = HandleMsg::UnbondGateway {};
        let result = handle(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(
            result,
            Err(ContractError::GatewayBondNotFound {
                account: "anyone".into()
            })
        );

        // let's add a node owned by bob
        helpers::add_gateway("bob", good_gateway_bond(), &mut deps);

        // attempt to unbond fred's node, which doesn't exist
        let info = mock_info("fred", &[]);
        let msg = HandleMsg::UnbondGateway {};
        let result = handle(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::GatewayBondNotFound {
                account: "fred".into()
            })
        );

        // bob's node is still there
        let nodes = helpers::get_gateways(&mut deps);
        assert_eq!(1, nodes.len());

        let first_node = &nodes[0];
        assert_eq!("bob", first_node.owner());

        // add a node owned by fred
        let fred_bond = good_gateway_bond();
        helpers::add_gateway("fred", fred_bond.clone(), &mut deps);

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_gateways(&mut deps).len());

        // unbond fred's node
        let info = mock_info("fred", &[]);
        let msg = HandleMsg::UnbondGateway {};
        let remove_fred = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr("address", "fred"),
            attr(
                "gateway_bond",
                format!("amount: {} {}, owner: fred", INITIAL_GATEWAY_BOND, DENOM),
            ),
        ];

        // we should see a funds transfer from the contract back to fred
        let expected_messages = vec![BankMsg::Send {
            from_address: env.contract.address,
            to_address: info.sender,
            amount: fred_bond,
        }
        .into()];

        // run the handler and check that we got back the correct results
        let expected = HandleResponse {
            messages: expected_messages,
            attributes: expected_attributes,
            data: None,
        };
        assert_eq!(remove_fred, expected);

        // only 1 node now exists, owned by bob:
        let gateway_bonds = helpers::get_gateways(&mut deps);
        assert_eq!(1, gateway_bonds.len());
        assert_eq!("bob", gateway_bonds[0].owner());
    }

    #[test]
    fn updating_state_params() {
        let mut deps = helpers::init_contract();

        let new_params = StateParams {
            epoch_length: 1,
            minimum_mixnode_bond: 123u128.into(),
            minimum_gateway_bond: 456u128.into(),
            mixnode_bond_reward_rate: "1.23".parse().unwrap(),
            gateway_bond_reward_rate: "4.56".parse().unwrap(),
            mixnode_active_set_size: 1000,
        };

        // cannot be updated from non-owner account
        let info = mock_info("not-the-creator", &[]);
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Err(ContractError::Unauthorized));

        // but works fine from the creator account
        let info = mock_info("creator", &[]);
        let res = try_update_state_params(deps.as_mut(), info, new_params.clone());
        assert_eq!(res, Ok(HandleResponse::default()));

        // and the state is actually updated
        let current_state = config_read(deps.as_ref().storage).load().unwrap();
        assert_eq!(current_state.params, new_params)
    }

    #[test]
    fn rewarding_mixnode() {
        let mut deps = helpers::init_contract();
        let current_state = config(deps.as_mut().storage).load().unwrap();

        // errors out if executed by somebody else than network monitor
        let info = mock_info("not-the-monitor", &[]);
        let res = try_reward_mixnode(deps.as_mut(), info, "node-owner".into(), 100);
        assert_eq!(res, Err(ContractError::Unauthorized));

        // errors out if the target owner hasn't bound any mixnodes
        let info = mock_info(NETWORK_MONITOR_ADDRESS, &[]);
        let res = try_reward_mixnode(deps.as_mut(), info, "node-owner".into(), 100);
        assert!(res.is_err());

        let initial_bond = 100_000000;
        let mixnode_bond = MixNodeBond {
            amount: coins(initial_bond, DENOM),
            owner: "node-owner".into(),
            mix_node: mix_node_fixture(),
        };

        mixnodes(deps.as_mut().storage)
            .save(b"node-owner", &mixnode_bond)
            .unwrap();

        let reward = read_mixnode_epoch_reward_rate(deps.as_ref().storage);

        // the node's bond is correctly increased and scaled by uptime
        // if node was 100% up, it will get full epoch reward
        let expected_bond = Uint128(initial_bond) * reward + Uint128(initial_bond);

        let info = mock_info(NETWORK_MONITOR_ADDRESS, &[]);
        try_reward_mixnode(deps.as_mut(), info, "node-owner".into(), 100);

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, b"node-owner").unwrap()
        );

        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_reward = scale_reward_by_uptime(reward, 20).unwrap();
        let expected_bond = expected_bond * reward + expected_bond;

        let info = mock_info(NETWORK_MONITOR_ADDRESS, &[]);
        try_reward_mixnode(deps.as_mut(), info, "node-owner".into(), 100);

        assert_eq!(
            expected_bond,
            read_mixnode_bond(deps.as_ref().storage, b"node-owner").unwrap()
        );
    }

    #[test]
    fn rewarding_gateway() {
        let mut deps = helpers::init_contract();
        let current_state = config(deps.as_mut().storage).load().unwrap();

        // errors out if executed by somebody else than network monitor
        let info = mock_info("not-the-monitor", &[]);
        let res = try_reward_gateway(deps.as_mut(), info, "node-owner".into(), 100);
        assert_eq!(res, Err(ContractError::Unauthorized));

        // errors out if the target owner hasn't bound any mixnodes
        let info = mock_info(NETWORK_MONITOR_ADDRESS, &[]);
        let res = try_reward_gateway(deps.as_mut(), info, "node-owner".into(), 100);
        assert!(res.is_err());

        let initial_bond = 100_000000;
        let gateway_bond = GatewayBond {
            amount: coins(initial_bond, DENOM),
            owner: "node-owner".into(),
            gateway: gateway_fixture(),
        };

        gateways(deps.as_mut().storage)
            .save(b"node-owner", &gateway_bond)
            .unwrap();

        let reward = read_gateway_epoch_reward_rate(deps.as_ref().storage);

        // the node's bond is correctly increased and scaled by uptime
        // if node was 100% up, it will get full epoch reward
        let expected_bond = Uint128(initial_bond) * reward + Uint128(initial_bond);

        let info = mock_info(NETWORK_MONITOR_ADDRESS, &[]);
        try_reward_gateway(deps.as_mut(), info, "node-owner".into(), 100);

        assert_eq!(
            expected_bond,
            read_gateway_bond(deps.as_ref().storage, b"node-owner").unwrap()
        );

        // if node was 20% up, it will get 1/5th of epoch reward
        let scaled_reward = scale_reward_by_uptime(reward, 20).unwrap();
        let expected_bond = expected_bond * reward + expected_bond;

        let info = mock_info(NETWORK_MONITOR_ADDRESS, &[]);
        try_reward_gateway(deps.as_mut(), info, "node-owner".into(), 100);

        assert_eq!(
            expected_bond,
            read_gateway_bond(deps.as_ref().storage, b"node-owner").unwrap()
        );
    }
}
