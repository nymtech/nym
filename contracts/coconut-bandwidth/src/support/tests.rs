// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub mod helpers {
    use crate::instantiate;
    use coconut_bandwidth_contract_common::msg::InstantiateMsg;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{Empty, MemoryStorage, OwnedDeps};

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }
}
