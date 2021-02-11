use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, Topology};
use crate::state::{config, config_read, MixNode, MixNodeBond, State};
use cosmwasm_std::{
    attr, coins, to_binary, BankMsg, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse,
    MessageInfo, StdResult,
};

/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let state = State {
        mix_node_bonds: vec![],
        owner: info.sender,
    };
    config(deps.storage).save(&state)?;
    Ok(InitResponse::default())
}

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

pub fn try_add_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    mix_node: MixNode,
) -> Result<HandleResponse, ContractError> {
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        let incoming = &info.sent_funds[0];

        // check that the denomination is correct
        if incoming.denom != "unym" {
            return Err(ContractError::WrongDenom {});
        }
        // check that we have at least 1000 nym in our bond
        if incoming.amount < coins(1000_000000, "unym")[0].amount {
            return Err(ContractError::InsufficientBond {});
        }

        let bond = MixNodeBond {
            amount: info.sent_funds,
            owner: info.sender,
            mix_node,
        };
        state.mix_node_bonds.push(bond);
        Ok(state)
    })?;

    Ok(HandleResponse::default())
}

pub fn try_remove_mixnode(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<HandleResponse, ContractError> {
    // load state
    let state = config(deps.storage).load()?;

    // find the bond
    let mixnode_bond = match state.mix_node_bonds.iter().find(|b| b.owner == info.sender) {
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
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        state.mix_node_bonds.retain(|mnb| mnb.owner != info.sender);
        Ok(state)
    })?;

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
    let state = config_read(deps.storage).load()?;
    Ok(Topology {
        mix_node_bonds: state.mix_node_bonds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&coins(2000, "unym"));
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
        assert_eq!(2000u128, query_balance(env.contract.address, deps));
    }

    #[cfg(test)]
    mod adding_a_mixnode {
        use super::*;

        #[test]
        fn works_if_1000_nym_are_sent() {
            let mut deps = mock_dependencies(&[]);
            let msg = InitMsg {};
            let info = mock_info("creator", &[]);
            let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

            let info = mock_info("anyone", &coins(1000_000000, "unym"));
            let msg = HandleMsg::RegisterMixnode {
                mix_node: mix_node_fixture(),
            };
            let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

            let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
            let topology: Topology = from_binary(&res).unwrap();
            assert_eq!(1, topology.mix_node_bonds.len());
            assert_eq!(
                mix_node_fixture().location,
                topology.mix_node_bonds[0].mix_node.location
            )
        }

        #[test]
        fn fails_if_less_than_1000_nym_are_sent() {
            let mut deps = mock_dependencies(&[]);
            let msg = InitMsg {};
            let info = mock_info("creator", &[]);
            let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

            let info = mock_info("anyone", &coins(999_999999, "unym"));
            let msg = HandleMsg::RegisterMixnode {
                mix_node: mix_node_fixture(),
            };
            let result = handle(deps.as_mut(), mock_env(), info, msg);
            assert_eq!(result, Err(ContractError::InsufficientBond {}));

            let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
            let topology: Topology = from_binary(&res).unwrap();
            assert_eq!(0, topology.mix_node_bonds.len());
        }
    }

    fn query_balance(
        address: HumanAddr,
        deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> u128 {
        let querier = deps.as_ref().querier;
        querier
            .query_balance(address, "unym")
            .unwrap()
            .amount
            .into()
    }

    fn mix_node_fixture() -> MixNode {
        MixNode {
            host: "mix.node.org".to_string(),
            layer: 1,
            location: "Sweden".to_string(),
            sphinx_key: "sphinx".to_string(),
            version: "0.10.0".to_string(),
        }
    }
}

// #[test]
// fn increment() {
//     let mut deps = mock_dependencies(&coins(2, "token"));
//     let msg = InitMsg { count: 17 };
//     let info = mock_info("creator", &coins(2, "token"));
//     let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
//     // beneficiary can release it
//     let info = mock_info("anyone", &coins(2, "token"));
//     let msg = HandleMsg::Increment {};
//     let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
//     // should increase counter by 1
//     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//     let value: CountResponse = from_binary(&res).unwrap();
//     assert_eq!(18, value.count);
// }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies(&coins(2, "token"));

//         let msg = InitMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let unauthorized_info = mock_info("anyone", &coins(2, "token"));
//         let msg = HandleMsg::Reset { count: 5 };
//         let res = handle(deps.as_mut(), mock_env(), unauthorized_info, msg);
//         match res {
//             Err(ContractError::Unauthorized {}) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_info = mock_info("creator", &coins(2, "token"));
//         let msg = HandleMsg::Reset { count: 5 };
//         let _res = handle(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//         // should now be 5
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
// }
