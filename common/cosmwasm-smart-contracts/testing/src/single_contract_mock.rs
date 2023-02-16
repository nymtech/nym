// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ContractState, TestableContract};
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{Env, MessageInfo, QueryResponse, Response};
use std::marker::PhantomData;

pub struct SingleContractMock<C> {
    pub state: ContractState,
    _contract: PhantomData<C>,
}

impl<C: TestableContract> SingleContractMock<C> {
    pub fn new_empty() -> Self {
        SingleContractMock {
            state: Default::default(),
            _contract: PhantomData,
        }
    }

    pub fn new(state: ContractState) -> Self {
        SingleContractMock {
            state,
            _contract: PhantomData,
        }
    }

    pub fn instantiate(
        custom_env: Option<Env>,
        info: MessageInfo,
        msg: C::InstantiateMsg,
    ) -> Result<(Self, Response), C::ContractError> {
        // if we're instantiating fresh contract it means there was no pre-existing state
        let env = custom_env.unwrap_or_else(mock_env);
        let state = ContractState::new_with_env(env);
        let mut this = Self::new(state);

        let env = this.state.env_cloned();
        let deps = this.state.deps_mut();

        let res = C::instantiate(deps, env, info, msg)?;
        Ok((this, res))
    }

    pub fn execute(
        &mut self,
        info: MessageInfo,
        msg: C::ExecuteMsg,
    ) -> Result<Response, C::ContractError> {
        let env = self.state.env_cloned();
        let deps = self.state.deps_mut();

        C::execute(deps, env, info, msg)
    }

    pub fn query(&self, msg: C::QueryMsg) -> Result<QueryResponse, C::ContractError> {
        let env = self.state.env_cloned();
        let deps = self.state.deps();

        C::query(deps, env, msg)
    }
}
