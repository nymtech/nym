use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::queries::{query_gateways_paged, query_mixnodes_paged};
use crate::state::{config, gateways, gateways_read, State};
use crate::{error::ContractError, state::mixnodes, state::mixnodes_read};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, HandleResponse,
    InitResponse, MessageInfo, StdResult, Uint128,
};
use mixnet_contract::{Gateway, GatewayBond, MixNode, MixNodeBond};

/// Constant specifying minimum of `unym` required to bond a gateway
const GATEWAY_BONDING_STAKE: Uint128 = Uint128(1000_000000); // 1000 nym

/// Constant specifying denomination of the coin used for bonding
const STAKE_DENOM: &str = "unym";

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let state = State { owner: info.sender };
    config(deps.storage).save(&state)?;
    Ok(InitResponse::default())
}

/// Handle an incoming message
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::RegisterMixnode { mix_node } => try_add_mixnode(deps, info, mix_node),
        HandleMsg::UnRegisterMixnode {} => try_remove_mixnode(deps, info, env),
        HandleMsg::BondGateway { gateway } => try_add_gateway(deps, info, gateway),
        HandleMsg::UnbondGateway {} => try_remove_gateway(deps, info, env),
    }
}

pub fn try_add_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    mix_node: MixNode,
) -> Result<HandleResponse, ContractError> {
    let stake = &info.sent_funds[0];

    // check that the denomination is correct
    if stake.denom != "unym" {
        return Err(ContractError::WrongDenom {});
    }
    // check that we have at least 1000 nym in our bond
    if stake.amount < coins(1000_000000, "unym")[0].amount {
        return Err(ContractError::InsufficientMixNodeBond {});
    }

    let bond = MixNodeBond::new(info.sent_funds, info.sender.clone(), mix_node);

    mixnodes(deps.storage).save(info.sender.as_bytes(), &bond)?;
    Ok(HandleResponse::default())
}

fn try_remove_mixnode(
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

fn validate_gateway_stake(stake: &[Coin]) -> Result<(), ContractError> {
    // check if anything was put as stake
    if stake.is_empty() {
        return Err(ContractError::NoStakeFound);
    }

    if stake.len() > 1 {
        // TODO: ask DH what would be an appropriate action here
    }

    // check that the denomination is correct
    if stake[0].denom != STAKE_DENOM {
        return Err(ContractError::WrongDenom {});
    }

    // check that we have at least 1000 nym in our bond
    if stake[0].amount < GATEWAY_BONDING_STAKE {
        return Err(ContractError::InsufficientGatewayBond {
            received: stake[0].amount.into(),
            minimum: GATEWAY_BONDING_STAKE.into(),
        });
    }

    Ok(())
}

pub(crate) fn try_add_gateway(
    deps: DepsMut,
    info: MessageInfo,
    gateway: Gateway,
) -> Result<HandleResponse, ContractError> {
    validate_gateway_stake(&info.sent_funds)?;

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

fn try_remove_gateway(
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

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetMixNodes { start_after, limit } => {
            to_binary(&query_mixnodes_paged(deps, start_after, limit)?)
        }
        QueryMsg::GetGateways { limit, start_after } => {
            to_binary(&query_gateways_paged(deps, start_after, limit)?)
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use mixnet_contract::{PagedGatewayResponse, PagedResponse};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let msg = InitMsg {};
        let info = mock_info("creator", &[]);

        let res = init(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mix_node_bonds should be empty after initialization
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.nodes.len()); // there are no mixnodes in the list when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, "unym"),
            query_contract_balance(env.contract.address, deps)
        );
    }

    #[test]
    fn mixnode_add() {
        let mut deps = helpers::init_contract();

        // if we don't send enough funds
        let info = mock_info("anyone", &coins(999_999999, "unym"));
        let msg = HandleMsg::RegisterMixnode {
            mix_node: helpers::mix_node_fixture(),
        };

        // we are informed that we didn't send enough funds
        let result = handle(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(result, Err(ContractError::InsufficientMixNodeBond {}));

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
        let info = mock_info("anyone", &coins(1000_000000, "unym"));
        let msg = HandleMsg::RegisterMixnode {
            mix_node: helpers::mix_node_fixture(),
        };

        // we get back a message telling us everything was OK
        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(HandleResponse::default(), handle_response);

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
        assert_eq!(&helpers::mix_node_fixture(), page.nodes[0].mix_node())

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
        let info = mock_info("anyone", &coins(999_9999, "unym"));
        let msg = HandleMsg::UnRegisterMixnode {};
        let result = handle(deps.as_mut(), mock_env(), info, msg);

        // we're told that there is no node for our address
        assert_eq!(result, Err(ContractError::MixNodeBondNotFound {}));

        // let's add a node owned by bob
        helpers::add_mixnode("bob", coins(1000_000000, "unym"), &mut deps);

        // attempt to un-register fred's node, which doesn't exist
        let info = mock_info("fred", &coins(999_9999, "unym"));
        let msg = HandleMsg::UnRegisterMixnode {};
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
        let fred_bond = coins(1666_000000, "unym");
        helpers::add_mixnode("fred", fred_bond.clone(), &mut deps);

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_mix_nodes(&mut deps).len());

        // un-register fred's node
        let info = mock_info("fred", &coins(999_9999, "unym"));
        let msg = HandleMsg::UnRegisterMixnode {};
        let remove_fred = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr(
                "mixnode_bond",
                "amount: [Coin { denom: \"unym\", amount: Uint128(1666000000) }], owner: fred",
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

    fn good_gateway_stake() -> Vec<Coin> {
        vec![Coin {
            denom: STAKE_DENOM.to_string(),
            amount: GATEWAY_BONDING_STAKE,
        }]
    }

    #[test]
    fn validating_gateway_stake() {
        // you must send SOME funds
        let result = validate_gateway_stake(&[]);
        assert_eq!(result, Err(ContractError::NoStakeFound));

        // you must send at least 1000 nym...
        let mut stake = good_gateway_stake();
        stake[0].amount = (GATEWAY_BONDING_STAKE - Uint128(1)).unwrap();
        let result = validate_gateway_stake(&stake);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: Into::<u128>::into(GATEWAY_BONDING_STAKE) - 1,
                minimum: GATEWAY_BONDING_STAKE.into(),
            })
        );

        // more than that is still fine
        let mut stake = good_gateway_stake();
        stake[0].amount = GATEWAY_BONDING_STAKE + Uint128(1);
        let result = validate_gateway_stake(&stake);
        assert!(result.is_ok());

        // it must be sent as unym!
        let mut stake = good_gateway_stake();
        stake[0].denom = "nym".to_string();
        let result = validate_gateway_stake(&stake);
        assert_eq!(result, Err(ContractError::WrongDenom {}));

        let mut stake = good_gateway_stake();
        stake[0].denom = "foomp".to_string();
        let result = validate_gateway_stake(&stake);
        assert_eq!(result, Err(ContractError::WrongDenom {}));
    }

    #[test]
    fn gateway_add() {
        let mut deps = helpers::init_contract();

        // if we fail validation (by say not sending enough funds
        let insufficient_bond = Into::<u128>::into(GATEWAY_BONDING_STAKE) - 1;
        let info = mock_info("anyone", &coins(insufficient_bond, STAKE_DENOM));
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        // we are informed that we didn't send enough funds
        let result = handle(deps.as_mut(), mock_env(), info, msg);
        assert_eq!(
            result,
            Err(ContractError::InsufficientGatewayBond {
                received: insufficient_bond,
                minimum: GATEWAY_BONDING_STAKE.into(),
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
        let info = mock_info("anyone", &good_gateway_stake());
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
        let info = mock_info("foomper", &good_gateway_stake());
        let msg = HandleMsg::BondGateway {
            gateway: helpers::gateway_fixture(),
        };

        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(handle_response.attributes[0], attr("overwritten", false));

        let info = mock_info("foomper", &good_gateway_stake());
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
        helpers::add_gateway("bob", good_gateway_stake(), &mut deps);

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
        let fred_bond = good_gateway_stake();
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
            attr("gateway_bond", "amount: 1000000000 unym, owner: fred"),
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
}
