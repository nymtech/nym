// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub mod helpers {
    pub const OWNER: &str = "admin0001";
    pub const SOMEBODY: &str = "somebody";
    pub const MULTISIG_CONTRACT: &str = "multisig contract address";
    pub const POOL_CONTRACT: &str = "mix pool contract address";

    use crate::contract::instantiate;
    use coconut_bandwidth_contract_common::msg::InstantiateMsg;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{Addr, Coin, Empty, MemoryStorage, OwnedDeps};
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper};

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            multisig_addr: String::from(MULTISIG_CONTRACT),
            pool_addr: String::from(POOL_CONTRACT),
        };
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }

    pub fn mock_app(init_funds: &[Coin]) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(OWNER), init_funds.to_vec())
                .unwrap();
        })
    }

    pub fn contract_bandwidth() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }
}
