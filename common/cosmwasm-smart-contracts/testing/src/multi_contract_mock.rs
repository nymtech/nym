// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract_mock::ContractState;
use crate::execution::{
    CrossContractTokenMove, ExecutionResult, ExecutionStepResult, FurtherExecution,
};
use crate::helpers::raw_msg_to_string;
use crate::traits::sealed;
use crate::{serialize_msg, MockingError, TestableContract};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    Addr, Binary, CosmosMsg, Env, MessageInfo, QueryResponse, ReplyOn, Response, WasmMsg,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

struct MockedContract {
    state: ContractState,
    entry_points: Box<dyn sealed::ErasedTestableContract>,
}

impl MockedContract {
    fn new<C: TestableContract + 'static>(state: ContractState) -> Self {
        MockedContract {
            state,
            entry_points: Box::new(C::new()),
        }
    }
}

#[derive(Default)]
pub struct MultiContractMock {
    contracts: HashMap<Addr, MockedContract>,
}

impl MultiContractMock {
    #[cfg(feature = "rand")]
    fn generate_new_contract_address(&self) -> Addr {
        use rand_chacha::rand_core::RngCore;

        let mut rng = crate::helpers::test_rng();
        loop {
            // for the testing purposes u64 contains enough entropy
            // (I could even argue u8 would be sufficient)
            // as I doubt anyone would want to generate so many contract names
            // they would have started colliding...
            let candidate_id = rng.next_u64();
            let name = Addr::unchecked(format!("new-contract{candidate_id}"));
            if !self.contracts.contains_key(&name) {
                return name;
            }
        }
    }

    pub fn new() -> Self {
        MultiContractMock {
            contracts: Default::default(),
        }
    }

    pub fn add_contract<C: TestableContract + 'static>(
        &mut self,
        contract_state: ContractState,
    ) -> Result<(), MockingError> {
        let address = contract_state.contract_address().clone();
        if self
            .contracts
            .contains_key(contract_state.contract_address())
        {
            Err(MockingError::DuplicateContractAddress { address })
        } else {
            let mocked = MockedContract::new::<C>(contract_state);
            self.contracts.insert(address, mocked);
            Ok(())
        }
    }

    pub fn with_contract<C: TestableContract + 'static>(
        mut self,
        state: ContractState,
    ) -> Result<Self, MockingError> {
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

    fn execute_step(
        &mut self,
        contract_address: impl Into<String>,
        info: MessageInfo,
        binary_msg: Binary,
    ) -> Result<ExecutionStepResult, MockingError> {
        let addr = Addr::unchecked(contract_address.into());
        let contract =
            self.contracts
                .get_mut(&addr)
                .ok_or_else(|| MockingError::NonExistentContract {
                    address: addr.clone(),
                })?;

        let env = contract.state.env_cloned();
        let deps = contract.state.deps_mut();

        let res = match contract
            .entry_points
            .execute(deps, env, info, binary_msg.clone())
        {
            Ok(res) => res,
            Err(error) => {
                return Err(MockingError::ContractExecutionError {
                    message: raw_msg_to_string(&binary_msg),
                    contract: addr,
                    error,
                })
            }
        };

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

        Ok(ExecutionStepResult {
            events: res.events,
            incoming_tokens,
            bank_msgs,
            further_execution,
        })
    }

    // TODO: verify that this is the actual order of execution of sub messages in cosmwasm
    fn execute_branch(
        &mut self,
        res: &mut ExecutionResult,
        contract: String,
        info: MessageInfo,
        msg: Binary,
    ) -> Result<(), MockingError> {
        let step_res = self.execute_step(contract.clone(), info, msg)?;
        res.steps.push(step_res.clone());
        for further_exec in step_res.further_execution {
            let info = mock_info(&contract, &further_exec.funds);
            self.execute_branch(
                res,
                further_exec.contract.into_string(),
                info,
                further_exec.msg,
            )?
        }
        Ok(())
    }

    pub fn contract_state(
        &self,
        contract_address: impl Into<String>,
    ) -> Result<&ContractState, MockingError> {
        let addr = Addr::unchecked(contract_address.into());
        let contract =
            self.contracts
                .get(&addr)
                .ok_or_else(|| MockingError::NonExistentContract {
                    address: addr.clone(),
                })?;
        Ok(&contract.state)
    }

    pub fn contract_state_mut(
        &mut self,
        contract_address: impl Into<String>,
    ) -> Result<&mut ContractState, MockingError> {
        let addr = Addr::unchecked(contract_address.into());
        let contract =
            self.contracts
                .get_mut(&addr)
                .ok_or_else(|| MockingError::NonExistentContract {
                    address: addr.clone(),
                })?;
        Ok(&mut contract.state)
    }

    // TODO: add support for sub msgs in instantiate response
    pub fn instantiate<C>(
        &mut self,
        custom_env: Option<Env>,
        info: MessageInfo,
        msg: C::InstantiateMsg,
    ) -> Result<Response, C::ContractError>
    where
        C: TestableContract + 'static,
    {
        // if custom environment wasn't provided, generate a pseudorandom address so that it wouldn't
        // clash with any existing contracts
        let env = custom_env.unwrap_or_else(|| {
            let mut env = mock_env();
            env.contract.address = self.generate_new_contract_address();
            env
        });
        let mut state = ContractState::new_with_env(env);
        let env = state.env_cloned();
        let deps = state.deps_mut();
        C::instantiate(deps, env, info, msg)
    }

    pub fn execute_full<C>(
        &mut self,
        initial_contract: impl Into<String>,
        info: MessageInfo,
        msg: C::ExecuteMsg,
    ) -> Result<ExecutionResult, MockingError>
    where
        C: TestableContract + 'static,
        C::ExecuteMsg: Serialize,
    {
        let mut execution_result = ExecutionResult::new();
        let serialized_msg = serialize_msg(&msg)?;

        self.execute_branch(
            &mut execution_result,
            initial_contract.into(),
            info,
            serialized_msg,
        )?;
        Ok(execution_result)
    }

    // provide unchecked variant of execute to return original error enum
    pub fn unchecked_execute<C>(
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
            .expect("specified contract does not exist");

        let env = contract.state.env_cloned();
        let deps = contract.state.deps_mut();
        C::execute(deps, env, info, msg)
    }

    // executes only the top level message
    pub fn execute<C>(
        &mut self,
        contract_address: impl Into<String>,
        info: MessageInfo,
        msg: C::ExecuteMsg,
    ) -> Result<Response, MockingError>
    where
        C: TestableContract + 'static,
        C::ExecuteMsg: Serialize,
    {
        let addr = Addr::unchecked(contract_address.into());
        let contract =
            self.contracts
                .get_mut(&addr)
                .ok_or_else(|| MockingError::NonExistentContract {
                    address: addr.clone(),
                })?;

        let env = contract.state.env_cloned();
        let deps = contract.state.deps_mut();

        let serialized_msg = serialize_msg(&msg)?;
        C::execute(deps, env, info, msg).map_err(|err| MockingError::ContractExecutionError {
            message: raw_msg_to_string(&serialized_msg),
            contract: addr,
            error: err.to_string(),
        })
    }

    // provide unchecked variant of query to return original error enum
    pub fn unchecked_query<C, T>(
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
            .expect("specified contract does not exist");

        let env = contract.state.env_cloned();
        let deps = contract.state.deps();
        C::query(deps, env, msg).map(|res| serde_json::from_slice(&res).unwrap())
    }

    pub fn query<C>(
        &self,
        contract_address: impl Into<String>,
        msg: C::QueryMsg,
    ) -> Result<QueryResponse, MockingError>
    where
        C: TestableContract + 'static,
        C::QueryMsg: Serialize,
    {
        let addr = Addr::unchecked(contract_address.into());
        let contract =
            self.contracts
                .get(&addr)
                .ok_or_else(|| MockingError::NonExistentContract {
                    address: addr.clone(),
                })?;

        let env = contract.state.env_cloned();
        let deps = contract.state.deps();

        let serialized_msg = serialize_msg(&msg)?;
        C::query(deps, env, msg).map_err(|err| MockingError::ContractQueryError {
            message: raw_msg_to_string(&serialized_msg),
            contract: addr,
            error: err.to_string(),
        })
    }

    pub fn query_de<C, T>(
        &self,
        contract_address: impl Into<String>,
        msg: C::QueryMsg,
    ) -> Result<T, MockingError>
    where
        C: TestableContract + 'static,
        C::QueryMsg: Serialize,
        T: DeserializeOwned,
    {
        self.query::<C>(contract_address, msg)
            .map(|res| serde_json::from_slice(&res).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn converting_msg_to_string() {
        #[derive(Serialize, Deserialize)]
        struct Dummy {
            field1: String,
            field2: u32,
            field3: Vec<u32>,
        }

        let dummy = Dummy {
            field1: "aaaa".to_string(),
            field2: 42,
            field3: vec![1, 2, 3, 4],
        };

        let bin = serialize_msg(&dummy).unwrap();
        let expected = r#"{"field1":"aaaa","field2":42,"field3":[1,2,3,4]}"#;
        let stringified = raw_msg_to_string(&bin);
        assert_eq!(expected, stringified)
    }
}
