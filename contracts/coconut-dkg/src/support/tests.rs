// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub mod helpers {
    use crate::instantiate;
    use coconut_dkg_common::msg::InstantiateMsg;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{Empty, MemoryStorage, OwnedDeps};

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {
            public_key_submission_end_height: env.block.height + 123,
            system_threshold: None,
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }
}
