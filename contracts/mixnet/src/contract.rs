use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, Topology};
use crate::state::{config, config_read, State};
use crate::types::MixNode;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, StdResult,
};
// use validator_client::models::mixnode::RegisteredMix;

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
pub fn init(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse, ContractError> {
    let state = State {
        mix_nodes: vec![],
        owner: deps.api.canonical_address(&info.sender)?,
    };
    config(deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    match msg {
        HandleMsg::RegisterMixnode { mix_node } => try_add_mixnode(deps, mix_node),
    }
}

pub fn try_add_mixnode(deps: DepsMut, mix_node: MixNode) -> Result<HandleResponse, ContractError> {
    config(deps.storage).update(|mut state| -> Result<_, ContractError> {
        state.mix_nodes.push(mix_node);
        Ok(state)
    })?;

    Ok(HandleResponse::default())
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTopology {} => to_binary(&query_get_topology(deps)?),
    }
}

fn query_get_topology(deps: Deps) -> StdResult<Topology> {
    let state = config_read(deps.storage).load()?;
    Ok(Topology {
        mix_nodes: state.mix_nodes,
    })
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary};

//     #[test]
//     fn initialize_contract() {
//         let mut deps = mock_dependencies(&coins(2000, "unym"));

//         let msg = InitMsg {};
//         let info = mock_info("creator", &coins(1000, "unym"));
//         // we can just call .unwrap() to assert this was a success
//         let res = init(deps.as_mut(), mock_env(), info, msg).unwrap();
//         println!("res is: {:?}", res);
//         // println!("FOO: {:?}", contract_address);

//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTopology {}).unwrap();
//         let topology: Topology = from_binary(&res).unwrap();
//         assert_eq!(0, topology.mix_nodes.len()); // there are no mixnodes in the topology when it's just been initialized

//         // OK, this is the question: how do I get the contract address so that I can then query to figure out what its balance is?
//         assert_eq!(
//             1000u128,
//             deps.as_ref()
//                 .querier
//                 .query_balance("creator", "unym")
//                 .unwrap()
//                 .amount
//                 .into()
//         );
//     }

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
//         let unauth_info = mock_info("anyone", &coins(2, "token"));
//         let msg = HandleMsg::Reset { count: 5 };
//         let res = handle(deps.as_mut(), mock_env(), unauth_info, msg);
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
