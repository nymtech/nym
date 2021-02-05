use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, Topology};
use crate::state::{config, config_read, State};
use crate::types::MixNode;
use crate::types::MixNodeBond;
use cosmwasm_std::coins;
use cosmwasm_std::BankMsg;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo,
    StdResult,
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
        None => return Err(ContractError::Unauthorized {}), // TODO: change to a more specific error type
        Some(bond) => bond,
    };
    // send bonded funds back to the bond owner
    let messages = vec![BankMsg::Send {
        from_address: env.contract.address.clone(),
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies(&coins(2000, "unym"));

        let msg = InitMsg {};
        let info = mock_info("creator", &coins(1000, "unym"));
        // we can just call .unwrap() to assert this was a success
        let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
        println!("res is: {:?}", res);
        // println!("FOO: {:?}", contract_address);

        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
        let topology: Topology = from_binary(&res).unwrap();
        assert_eq!(0, topology.mix_node_bonds.len()); // there are no mixnodes in the topology when it's just been initialized

        // OK, this is the question: how do I get the contract address so that I can then query to figure out what its balance is?
        assert_eq!(
            1000u128,
            deps.as_ref()
                .querier
                .query_balance("creator", "unym")
                .unwrap()
                .amount
                .into()
        );
    }
}

//     #[cfg(test)]
//     mod adding_a_mixnode {
//         // use super::*;

//         // #[test]
//         // fn works() {
//         //     let mut deps = mock_dependencies(&coins(2, "token"));
//         //     let msg = InitMsg {};
//         //     let info = mock_info("creator", &coins(2, "token"));
//         //     let _res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
//         //     // beneficiary can release it
//         //     let info = mock_info("anyone", &coins(2, "token"));
//         //     let msg = HandleMsg::Increment {};
//         //     let _res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();
//         //     // should increase counter by 1
//         //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         //     let value: CountResponse = from_binary(&res).unwrap();
//         //     assert_eq!(18, value.count);
//         // }
//     }
// }

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
