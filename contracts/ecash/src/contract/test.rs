// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi, MockQuerier};
use cosmwasm_std::{coin, Addr, Empty, Env, MemoryStorage, OwnedDeps};
use sylvia::ctx::{InstantiateCtx, QueryCtx};

pub const TEST_DENOM: &str = "unym";

#[allow(dead_code)]
pub struct TestSetup {
    pub contract: NymEcashContract,
    pub deps: OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
    pub env: Env,

    pub holding_account: Addr,
    pub multisig_contract: Addr,
    pub group_contract: Addr,
}

impl TestSetup {
    pub fn init() -> TestSetup {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let admin = message_info(&deps.api.addr_make("admin"), &[]);
        let multisig_contract = deps.api.addr_make("multisig");
        let group_contract = deps.api.addr_make("group");
        let holding = deps.api.addr_make("holding");

        let init_ctx = InstantiateCtx::from((deps.as_mut(), env.clone(), admin));

        let contract = NymEcashContract::new();
        contract
            .instantiate(
                init_ctx,
                holding.to_string(),
                multisig_contract.to_string(),
                group_contract.to_string(),
                coin(75000000, TEST_DENOM),
            )
            .unwrap();

        TestSetup {
            contract,
            deps,
            env,
            holding_account: Addr::unchecked(holding),
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
