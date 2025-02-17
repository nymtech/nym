// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use crate::helpers::Config;
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi, MockQuerier};
use cosmwasm_std::{coin, Addr, Deps, Empty, Env, MemoryStorage, MessageInfo, OwnedDeps};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sylvia::ctx::{ExecCtx, InstantiateCtx, QueryCtx};

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

const CONTRACT: NymEcashContract = NymEcashContract::new();

const DENOM: &str = "unym";

#[allow(dead_code)]
pub struct TestSetupSimple {
    pub deps: OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
    pub env: Env,
    pub rng: ChaCha20Rng,
    pub owner: Addr,
    pub holding_account: Addr,
    pub multisig_contract: Addr,
    pub group_contract: Addr,
}

impl TestSetupSimple {
    pub fn new() -> Self {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner = Addr::unchecked("owner");

        let init_ctx = InstantiateCtx::from((
            deps.as_mut(),
            env.clone(),
            message_info(owner.as_str(), &[]),
        ));

        let rng = test_rng();
        let holding_account = Addr::unchecked("holding_account");
        let multisig_contract = Addr::unchecked("multisig_contract");
        let group_contract = Addr::unchecked("group_contract");

        CONTRACT
            .instantiate(
                init_ctx,
                holding_account.to_string(),
                multisig_contract.to_string(),
                group_contract.to_string(),
                coin(75_000_000, DENOM.to_string()),
            )
            .unwrap();

        TestSetupSimple {
            deps,
            env,
            rng,
            owner,
            holding_account,
            multisig_contract,
            group_contract,
        }
    }

    pub fn admin(&self) -> MessageInfo {
        let admin = CONTRACT
            .contract_admin
            .get(self.deps.as_ref())
            .unwrap()
            .unwrap();
        message_info(admin.as_str(), &[])
    }

    pub fn execute_ctx(&mut self, sender: MessageInfo) -> ExecCtx {
        let env = self.env.clone();
        ExecCtx::from((self.deps.as_mut(), env, sender))
    }

    #[allow(dead_code)]
    pub fn query_ctx(&self) -> QueryCtx {
        QueryCtx::from((self.deps.as_ref(), self.env.clone()))
    }

    pub fn contract(&self) -> NymEcashContract {
        CONTRACT
    }

    pub fn deps(&self) -> Deps<'_> {
        self.deps.as_ref()
    }

    pub fn config(&self) -> Config {
        CONTRACT.config.load(self.deps().storage).unwrap()
    }

    pub fn with_deposit_amount(mut self, amount: u128) -> Self {
        CONTRACT
            .update_deposit_value(self.execute_ctx(self.admin()), coin(amount, DENOM))
            .unwrap();
        self
    }
}
