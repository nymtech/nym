// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{Addr, Empty, Env, MemoryStorage, OwnedDeps};
use sylvia::types::{InstantiateCtx, QueryCtx};

pub const TEST_DENOM: &str = "unym";

pub struct TestSetup {
    pub contract: NymEcashContract<'static>,
    pub deps: OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
    pub env: Env,

    pub multisig_contract: Addr,
    pub group_contract: Addr,
}

impl TestSetup {
    pub fn init() -> TestSetup {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin = mock_info("admin", &[]);
        let init_ctx = InstantiateCtx::from((deps.as_mut(), env.clone(), admin));

        let multisig_contract = "multisig";
        let group_contract = "group";

        let contract = NymEcashContract::new();
        contract
            .instantiate(
                init_ctx,
                multisig_contract.to_string(),
                group_contract.to_string(),
                TEST_DENOM.to_string(),
            )
            .unwrap();

        TestSetup {
            contract,
            deps,
            env,
            multisig_contract: Addr::unchecked(multisig_contract),
            group_contract: Addr::unchecked(group_contract),
        }
    }

    pub fn query_ctx(&self) -> QueryCtx {
        QueryCtx::from((self.deps.as_ref(), self.env.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_ecash_contract_common::deposit::Deposit;
    use sylvia::anyhow;

    #[test]
    fn deposit_queries() -> anyhow::Result<()> {
        let mut test = TestSetup::init();

        // no deposit
        let res = test.contract.get_deposit(test.query_ctx(), 42)?;
        assert!(res.deposit.is_none());

        let deps = test.deps.as_mut();
        let deposit_id = test.contract.deposits.save_deposit(
            deps.storage,
            "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".to_string(),
        )?;

        // deposit exists
        let res = test.contract.get_deposit(test.query_ctx(), deposit_id)?;
        let expected = Deposit {
            bs58_encoded_ed25519_pubkey: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".to_string(),
        };

        assert_eq!(expected, res.deposit.unwrap());

        Ok(())
    }
}
