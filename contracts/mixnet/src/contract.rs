use crate::msg::{HandleMsg, InitMsg, QueryMsg, Topology};
use crate::state::{config, MixNode, MixNodeBond, State};
use crate::{error::ContractError, state::mixnodes, state::mixnodes_all, state::mixnodes_read};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, StdResult,
};

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
    }
}

fn try_add_mixnode(
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
        return Err(ContractError::InsufficientBond {});
    }

    let bond = MixNodeBond {
        amount: info.sent_funds,
        owner: info.sender.clone(),
        mix_node,
    };

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
        amount: mixnode_bond.amount.clone(),
    }
    .into()];

    // remove the bond from the list of bonded mixnodes
    mixnodes(deps.storage).remove(mixnode_bond.owner.as_bytes());

    // log our actions
    let attributes = vec![
        attr("action", "unbond"),
        attr("tokens", mixnode_bond.amount[0].amount),
        attr("account", mixnode_bond.owner.clone()),
    ];

    Ok(HandleResponse {
        messages,
        attributes,
        data: None,
    })
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTopology {} => to_binary(&query_get_topology(deps)?),
    }
}

fn query_get_topology(deps: Deps) -> StdResult<Topology> {
    let mix_nodes = mixnodes_all(deps.storage)?;
    Ok(Topology {
        mix_node_bonds: mix_nodes,
    })
}

#[cfg(test)]
mod tests {
    use crate::state::mixnodes;

    use super::*;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{coins, from_binary};
    use cosmwasm_std::{testing::MockApi, Coin};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let msg = InitMsg {};
        let info = mock_info("creator", &[]);

        let res = init(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mix_node_bonds should be empty after initialization
        let res = query(deps.as_ref(), env.clone(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&res).unwrap();
        assert_eq!(0, topology.mix_node_bonds.len()); // there are no mixnodes in the topology when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, "unym"),
            helpers::query_contract_balance(env.contract.address, deps)
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
        assert_eq!(result, Err(ContractError::InsufficientBond {}));

        // no mixnode was inserted into the topology
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&res).unwrap();
        assert_eq!(0, topology.mix_node_bonds.len());

        // if we send enough funds
        let info = mock_info("anyone", &coins(1000_000000, "unym"));
        let msg = HandleMsg::RegisterMixnode {
            mix_node: helpers::mix_node_fixture(),
        };

        // we get back a message telling us everything was OK
        let handle_response = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(HandleResponse::default(), handle_response);

        // we can query topology and the new node is there
        let query_response = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&query_response).unwrap();
        assert_eq!(1, topology.mix_node_bonds.len());
        assert_eq!(
            helpers::mix_node_fixture().location,
            topology.mix_node_bonds[0].mix_node.location
        )
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
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&res).unwrap();
        let first_node = &topology.mix_node_bonds[0];
        assert_eq!(1, topology.mix_node_bonds.len());
        assert_eq!("bob", first_node.owner);

        // add a node owned by fred
        let fred_bond = coins(1666_000000, "unym");
        helpers::add_mixnode("fred", fred_bond.clone(), &mut deps);

        // let's make sure we now have 2 nodes:
        assert_eq!(2, helpers::get_topology(&mut deps).mix_node_bonds.len());

        // un-register fred's node
        let info = mock_info("fred", &coins(999_9999, "unym"));
        let msg = HandleMsg::UnRegisterMixnode {};
        let remove_fred = handle(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // we should see log messages come back showing an unbond message
        let expected_attributes = vec![
            attr("action", "unbond"),
            attr("tokens", fred_bond.clone()[0].amount),
            attr("account", "fred"),
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
        let topology = helpers::get_topology(&mut deps);
        assert_eq!(1, topology.mix_node_bonds.len());
        assert_eq!("bob", topology.mix_node_bonds[0].owner);
    }

    #[test]
    fn query_mixnodes() {
        let mut deps = helpers::init_contract();

        let result = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&result).unwrap();
        let nodes: Vec<MixNodeBond> = topology.mix_node_bonds;
        assert_eq!(0, nodes.len());

        // let's add a node
        let node = MixNodeBond {
            amount: coins(50, "unym"),
            owner: HumanAddr::from("foo"),
            mix_node: helpers::mix_node_fixture(),
        };
        mixnodes(&mut deps.storage)
            .save("foo".as_bytes(), &node)
            .unwrap();

        // is the node there?
        let result = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&result).unwrap();
        let nodes: Vec<MixNodeBond> = topology.mix_node_bonds;
        assert_eq!(1, nodes.len());
        assert_eq!(helpers::mix_node_fixture(), nodes[0].mix_node);
    }

    mod helpers {
        use super::*;
        use cosmwasm_std::{Empty, MemoryStorage};

        pub fn add_mixnode(
            pubkey: &str,
            stake: Vec<Coin>,
            deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
        ) {
            let info = mock_info(pubkey, &stake);
            try_add_mixnode(deps.as_mut(), info, helpers::mix_node_fixture()).unwrap();
        }

        pub fn get_topology(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) -> Topology {
            let result = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
            let topology: Topology = from_binary(&result).unwrap();
            topology
        }

        pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
            let mut deps = mock_dependencies(&[]);
            let msg = InitMsg {};
            let env = mock_env();
            let info = mock_info("creator", &[]);
            init(deps.as_mut(), env.clone(), info, msg).unwrap();
            return deps;
        }

        pub fn mix_node_fixture() -> MixNode {
            MixNode {
                host: "mix.node.org".to_string(),
                layer: 1,
                location: "Sweden".to_string(),
                sphinx_key: "sphinx".to_string(),
                version: "0.10.0".to_string(),
            }
        }

        pub fn query_contract_balance(
            address: HumanAddr,
            deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
        ) -> Vec<Coin> {
            let querier = deps.as_ref().querier;
            vec![querier.query_balance(address, "unym").unwrap()]
        }
    }
}
