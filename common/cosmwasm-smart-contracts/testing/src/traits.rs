// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QueryResponse, Response};
use serde::de::DeserializeOwned;

// TODO: see if it's possible to create a macro to auto-derive it
// if you intend to use the MultiContractMock, you need to implement this trait
// for your contract
/// ```
/// use cosmwasm_std::{
///     entry_point, Deps, DepsMut, Env, MessageInfo, Querier, QueryResponse, Response, StdError,
///     Storage,
/// };
/// use cosmwasm_contract_testing::TestableContract;
///
/// type ExecuteMsg = ();
/// type QueryMsg = ();
/// type InstantiateMsg = ();
/// type ContractError = StdError;
///
/// #[entry_point]
/// pub fn instantiate (
///     deps: DepsMut,
///     env: Env,
///     info: MessageInfo,
///     msg: InstantiateMsg,
/// ) -> Result<Response, ContractError> {
///     Ok(Default::default())
/// }
///
/// #[entry_point]
/// pub fn execute(
///     deps: DepsMut,
///     env: Env,
///     info: MessageInfo,
///     msg: ExecuteMsg,
/// ) -> Result<Response, ContractError> {
///     Ok(Default::default())
/// }
///
/// #[entry_point]
/// pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
///     Ok(Default::default())
/// }
///
/// struct MyContract;
///
/// impl TestableContract for MyContract {
///     type ContractError = ContractError;
///     type InstantiateMsg = InstantiateMsg;
///     type ExecuteMsg = ExecuteMsg;
///     type QueryMsg = QueryMsg;
///
///     fn new() -> Self {
///         MyContract
///     }
///
///     fn instantiate(
///         deps: DepsMut<'_>,
///         env: Env,
///         info: MessageInfo,
///         msg: Self::InstantiateMsg,
///     ) -> Result<Response, Self::ContractError> {
///         instantiate(deps, env, info, msg)
///     }
///
///     fn execute(
///         deps: DepsMut<'_>,
///         env: Env,
///         info: MessageInfo,
///         msg: Self::ExecuteMsg,
///     ) -> Result<Response, Self::ContractError> {
///         execute(deps, env, info, msg)
///     }
///
///     fn query(
///         deps: Deps<'_>,
///         env: Env,
///         msg: Self::QueryMsg,
///     ) -> Result<QueryResponse, Self::ContractError> {
///         query(deps, env, msg)
///     }
/// }
/// ```
pub trait TestableContract {
    type ContractError: ToString;
    type InstantiateMsg: DeserializeOwned;
    type ExecuteMsg: DeserializeOwned;
    type QueryMsg: DeserializeOwned;

    fn new() -> Self;

    fn instantiate(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::InstantiateMsg,
    ) -> Result<Response, Self::ContractError>;

    fn execute(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::ExecuteMsg,
    ) -> Result<Response, Self::ContractError>;

    fn query(
        deps: Deps<'_>,
        env: Env,
        msg: Self::QueryMsg,
    ) -> Result<QueryResponse, Self::ContractError>;
}

pub(crate) mod sealed {
    use crate::deserialize_msg;
    use crate::traits::TestableContract;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response};

    pub(crate) trait ErasedTestableContract {
        fn query(&self, deps: Deps<'_>, env: Env, raw_msg: Binary)
            -> Result<QueryResponse, String>;

        fn execute(
            &self,
            deps: DepsMut<'_>,
            env: Env,
            info: MessageInfo,
            raw_msg: Binary,
        ) -> Result<Response, String>;

        fn instantiate(
            &self,
            deps: DepsMut<'_>,
            env: Env,
            info: MessageInfo,
            raw_msg: Binary,
        ) -> Result<Response, String>;
    }

    impl<T: TestableContract> ErasedTestableContract for T {
        fn query(
            &self,
            deps: Deps<'_>,
            env: Env,
            raw_msg: Binary,
        ) -> Result<QueryResponse, String> {
            let msg = deserialize_msg(&raw_msg).expect("failed to deserialize 'QueryMsg'");
            <Self as TestableContract>::query(deps, env, msg).map_err(|err| err.to_string())
        }

        fn execute(
            &self,
            deps: DepsMut<'_>,
            env: Env,
            info: MessageInfo,
            raw_msg: Binary,
        ) -> Result<Response, String> {
            let msg = deserialize_msg(&raw_msg).expect("failed to deserialize 'ExecuteMsg'");
            <Self as TestableContract>::execute(deps, env, info, msg).map_err(|err| err.to_string())
        }

        fn instantiate(
            &self,
            deps: DepsMut<'_>,
            env: Env,
            info: MessageInfo,
            raw_msg: Binary,
        ) -> Result<Response, String> {
            let msg = deserialize_msg(&raw_msg).expect("failed to deserialize 'InstantiateMsg'");
            <Self as TestableContract>::instantiate(deps, env, info, msg)
                .map_err(|err| err.to_string())
        }
    }
}
