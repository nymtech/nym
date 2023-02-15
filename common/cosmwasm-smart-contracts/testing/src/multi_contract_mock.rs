// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_mock::ContractMock;
use crate::execution::{
    CrossContractTokenMove, ExecutionResult, ExecutionStepResult, FurtherExecution,
};
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, QueryResponse, ReplyOn, Response,
    StdResult, WasmMsg,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("attempted to add another contract mock that has the same address as an existing one - {address}")]
pub struct DuplicateContractAddress {
    address: Addr,
}

// TODO: see if it's possible to create a macro to auto-derive it
// if you intend to use the MultiContractMock, you need to implement this trait
// for your contract
/// ```
/// use cosmwasm_std::{
///     entry_point, Deps, DepsMut, Env, MessageInfo, Querier, QueryResponse, Response, StdError,
///     Storage,
/// };
/// use testing::TestableContract;
///
/// type ExecuteMsg = ();
/// type QueryMsg = ();
/// type ContractError = StdError;
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
///     type ExecuteMsg = ExecuteMsg;
///     type QueryMsg = QueryMsg;
///
///     fn new() -> Self {
///         MyContract
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
    // TODO: can we avoid the extra `Serialize` trait bound here?
    type ExecuteMsg: DeserializeOwned + Serialize;
    type QueryMsg: DeserializeOwned;

    fn new() -> Self;

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

struct MockedContract {
    state: ContractMock,
    handlers: Box<dyn sealed::ErasedTestableContract>,
}

impl MockedContract {
    fn new<C: TestableContract + 'static>(state: ContractMock) -> Self {
        MockedContract {
            state,
            handlers: Box::new(C::new()),
        }
    }
}

#[derive(Default)]
pub struct MultiContractMock {
    contracts: HashMap<Addr, MockedContract>,
}

impl MultiContractMock {
    pub fn new() -> Self {
        MultiContractMock {
            contracts: Default::default(),
        }
    }

    pub fn add_contract<C: TestableContract + 'static>(
        &mut self,
        contract_state: ContractMock,
    ) -> Result<(), DuplicateContractAddress> {
        let address = contract_state.contract_address().clone();
        if self
            .contracts
            .contains_key(contract_state.contract_address())
        {
            Err(DuplicateContractAddress { address })
        } else {
            let mocked = MockedContract::new::<C>(contract_state);
            self.contracts.insert(address, mocked);
            Ok(())
        }
    }

    pub fn with_contract<C: TestableContract + 'static>(
        mut self,
        state: ContractMock,
    ) -> Result<Self, DuplicateContractAddress> {
        self.add_contract::<C>(state)?;
        Ok(self)
    }

    pub fn advance_blocks(&mut self, new_blocks: u64) {
        for contract in self.contracts.values_mut() {
            contract.state.advance_blocks(new_blocks)
        }
    }

    pub fn advance_block_height(&mut self, by: u64) {
        for contract in self.contracts.values_mut() {
            contract.state.advance_block_height(by)
        }
    }

    pub fn advance_blocktime(&mut self, by_secs: u64) {
        for contract in self.contracts.values_mut() {
            contract.state.advance_blocktime(by_secs)
        }
    }

    // TODO: incorporate error handling...
    fn _execute_step(
        &mut self,
        contract_address: impl Into<String>,
        info: MessageInfo,
        binary_msg: Binary,
    ) -> ExecutionStepResult {
        let addr = Addr::unchecked(contract_address.into());
        let contract = self
            .contracts
            .get_mut(&addr)
            .expect("TODO: error handling; contract doesnt exist");

        let env = contract.state.env_cloned();
        let deps = contract.state.deps_mut();

        let res = contract
            .handlers
            .execute(deps, env, info, binary_msg)
            .unwrap();

        let mut bank_msgs = Vec::new();
        let mut further_execution = Vec::new();
        let mut incoming_tokens = Vec::new();

        for sub_msg in res.messages {
            if sub_msg.reply_on != ReplyOn::Never {
                unimplemented!("currently there's no support for 'reply_on'")
            }

            match sub_msg.msg {
                CosmosMsg::Bank(bank_msg) => bank_msgs.push(bank_msg),
                CosmosMsg::Wasm(wasm_msg) => {
                    match wasm_msg {
                        WasmMsg::Execute { contract_addr, msg, funds } => {
                            incoming_tokens.push(CrossContractTokenMove::new(funds.clone(), addr.clone(), Addr::unchecked(&contract_addr)));
                            further_execution.push(FurtherExecution::new(contract_addr, msg, funds))
                        }
                        _ => unimplemented!("currently we only support 'ExecuteMsg' for 'WasmMsg'")
                    }
                }
                // other variants might get support later on
                _ =>  unimplemented!("currently there's no support for sub msgs different from 'WasmMsg' or 'BankMsg")
            }
        }

        ExecutionStepResult {
            events: res.events,
            incoming_tokens,
            bank_msgs,
            further_execution,
        }
    }

    // TODO: verify that this is the actual order of execution of sub messages in cosmwasm
    fn execute_branch(
        &mut self,
        res: &mut ExecutionResult,
        contract: String,
        info: MessageInfo,
        msg: Binary,
    ) {
        let step_res = self._execute_step(contract.clone(), info, msg);
        res.steps.push(step_res.clone());
        for further_exec in step_res.further_execution {
            let info = mock_info(&contract, &further_exec.funds);
            self.execute_branch(
                res,
                further_exec.contract.into_string(),
                info,
                further_exec.msg,
            )
        }
    }

    pub fn execute_full<C>(
        &mut self,
        initial_contract: impl Into<String>,
        info: MessageInfo,
        msg: C::ExecuteMsg,
    ) -> Result<ExecutionResult, String>
    where
        C: TestableContract + 'static,
    {
        let mut execution_result = ExecutionResult::new();
        let serialized_msg = serialize_msg(&msg).unwrap();

        self.execute_branch(
            &mut execution_result,
            initial_contract.into(),
            info,
            serialized_msg,
        );
        Ok(execution_result)
    }

    // executes only the top level message
    pub fn execute<C>(
        &mut self,
        contract_address: impl Into<String>,
        info: MessageInfo,
        msg: C::ExecuteMsg,
    ) -> Result<Response, C::ContractError>
    where
        C: TestableContract + 'static,
    {
        let addr = Addr::unchecked(contract_address.into());
        let contract = self
            .contracts
            .get_mut(&addr)
            .expect("TODO: error handling; contract doesnt exist");

        let env = contract.state.env_cloned();
        let deps = contract.state.deps_mut();

        C::execute(deps, env, info, msg)
    }

    pub fn query<C, T>(
        &self,
        contract_address: impl Into<String>,
        msg: C::QueryMsg,
    ) -> Result<T, C::ContractError>
    where
        C: TestableContract + 'static,
        T: DeserializeOwned,
    {
        let addr = Addr::unchecked(contract_address.into());
        let contract = self
            .contracts
            .get(&addr)
            .expect("TODO: error handling; contract doesnt exist");

        let env = contract.state.env_cloned();
        let deps = contract.state.deps();

        C::query(deps, env, msg).map(|res| serde_json::from_slice(&res).unwrap())
    }
}

fn deserialize_msg<M: DeserializeOwned>(raw: Binary) -> StdResult<M> {
    cosmwasm_std::from_binary(&raw)
}

fn serialize_msg<M: Serialize>(msg: &M) -> StdResult<Binary> {
    cosmwasm_std::to_binary(msg)
}

pub(crate) mod sealed {
    use crate::multi_contract_mock::{deserialize_msg, TestableContract};
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
    }

    impl<T: TestableContract> ErasedTestableContract for T {
        fn query(
            &self,
            deps: Deps<'_>,
            env: Env,
            raw_msg: Binary,
        ) -> Result<QueryResponse, String> {
            let msg = deserialize_msg(raw_msg).expect("failed to deserialize 'QueryMsg'");
            <Self as TestableContract>::query(deps, env, msg).map_err(|err| err.to_string())
        }

        fn execute(
            &self,
            deps: DepsMut<'_>,
            env: Env,
            info: MessageInfo,
            raw_msg: Binary,
        ) -> Result<Response, String> {
            let msg = deserialize_msg(raw_msg).expect("failed to deserialize 'ExecuteMsg'");
            <Self as TestableContract>::execute(deps, env, info, msg).map_err(|err| err.to_string())
        }
    }
}
