// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::NymEcashContract;
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi, MockQuerier};
use cosmwasm_std::{coin, Addr, Empty, Env, MemoryStorage, MessageInfo, OwnedDeps};
use sylvia::ctx::{ExecCtx, InstantiateCtx, QueryCtx};

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

    pub fn query_ctx(&self) -> QueryCtx<'_> {
        QueryCtx::from((self.deps.as_ref(), self.env.clone()))
    }

    pub fn exec_ctx(&mut self, sender: MessageInfo) -> ExecCtx<'_> {
        ExecCtx::from((self.deps.as_mut(), self.env.clone(), sender))
    }

    pub fn admin_info(&self) -> MessageInfo {
        let admin = self
            .contract
            .contract_admin
            .get(self.deps.as_ref())
            .unwrap()
            .unwrap();
        message_info(&admin, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coin;
    use nym_ecash_contract_common::deposit::Deposit;
    use nym_ecash_contract_common::reduced_deposit::WhitelistedAccount;
    use nym_ecash_contract_common::EcashContractError;
    use sylvia::anyhow;

    const CONTRACT: NymEcashContract = NymEcashContract::new();

    #[test]
    fn deposit_queries() -> anyhow::Result<()> {
        let mut test = TestSetup::init();

        // no deposit
        let res = CONTRACT.get_deposit(test.query_ctx(), 42)?;
        assert!(res.deposit.is_none());

        let deposit_id = CONTRACT
            .deposits
            .save_deposit(
                test.deps.as_mut().storage,
                "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".to_string(),
            )?;

        // deposit exists
        let res = CONTRACT.get_deposit(test.query_ctx(), deposit_id)?;
        let expected = Deposit {
            bs58_encoded_ed25519_pubkey: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK".to_string(),
        };

        assert_eq!(expected, res.deposit.unwrap());

        Ok(())
    }

    #[test]
    fn get_default_deposit_amount_returns_configured_value() -> anyhow::Result<()> {
        let test = TestSetup::init();

        let amount = CONTRACT.get_default_deposit_amount(test.query_ctx())?;

        assert_eq!(amount, coin(75_000_000, TEST_DENOM));

        Ok(())
    }

    #[test]
    fn get_reduced_deposit_amount_returns_none_for_unlisted_address() -> anyhow::Result<()> {
        let test = TestSetup::init();
        let unknown = test.deps.api.addr_make("unknown");

        let amount =
            CONTRACT.get_reduced_deposit_amount(test.query_ctx(), unknown.to_string())?;

        assert!(amount.is_none());

        Ok(())
    }

    #[test]
    fn get_reduced_deposit_amount_returns_amount_for_whitelisted_address() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("whitelisted");
        let reduced = coin(10_000_000, TEST_DENOM);

        CONTRACT
            .reduced_deposits
            .save(test.deps.as_mut().storage, addr.clone(), &reduced)?;

        let amount =
            CONTRACT.get_reduced_deposit_amount(test.query_ctx(), addr.to_string())?;

        assert_eq!(amount, Some(reduced));

        Ok(())
    }

    // --- get_all_whitelisted_accounts ---

    #[test]
    fn get_all_whitelisted_accounts_returns_empty_by_default() -> anyhow::Result<()> {
        let test = TestSetup::init();

        let res = CONTRACT.get_all_whitelisted_accounts(test.query_ctx())?;
        assert!(res.whitelisted_accounts.is_empty());

        Ok(())
    }

    #[test]
    fn get_all_whitelisted_accounts_returns_all_entries() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let alice = test.deps.api.addr_make("alice");
        let bob = test.deps.api.addr_make("bob");

        let admin = test.admin_info();
        CONTRACT.set_reduced_deposit_price(
            test.exec_ctx(admin),
            alice.to_string(),
            coin(10_000_000, TEST_DENOM),
        )?;

        let admin = test.admin_info();
        CONTRACT.set_reduced_deposit_price(
            test.exec_ctx(admin),
            bob.to_string(),
            coin(5_000_000, TEST_DENOM),
        )?;

        let res = CONTRACT.get_all_whitelisted_accounts(test.query_ctx())?;
        assert_eq!(res.whitelisted_accounts.len(), 2);

        assert!(res.whitelisted_accounts.contains(&WhitelistedAccount {
            address: alice,
            deposit: coin(10_000_000, TEST_DENOM),
        }));
        assert!(res.whitelisted_accounts.contains(&WhitelistedAccount {
            address: bob,
            deposit: coin(5_000_000, TEST_DENOM),
        }));

        Ok(())
    }

    // --- set_reduced_deposit_price ---

    #[test]
    fn set_reduced_deposit_price_requires_admin() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let non_admin = test.deps.api.addr_make("non_admin");
        let addr = test.deps.api.addr_make("alice");

        let err = CONTRACT
            .set_reduced_deposit_price(
                test.exec_ctx(message_info(&non_admin, &[])),
                addr.to_string(),
                coin(10_000_000, TEST_DENOM),
            )
            .unwrap_err();

        assert!(matches!(err, EcashContractError::Admin(_)));

        Ok(())
    }

    #[test]
    fn set_reduced_deposit_price_rejects_wrong_denom() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");
        let admin = test.admin_info();

        let err = CONTRACT
            .set_reduced_deposit_price(
                test.exec_ctx(admin),
                addr.to_string(),
                coin(10_000_000, "uatom"),
            )
            .unwrap_err();

        assert_eq!(
            err,
            EcashContractError::InvalidReducedDepositDenom {
                expected: TEST_DENOM.to_string(),
                got: "uatom".to_string(),
            }
        );

        Ok(())
    }

    #[test]
    fn set_reduced_deposit_price_rejects_amount_equal_to_default() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");
        let admin = test.admin_info();

        let err = CONTRACT
            .set_reduced_deposit_price(
                test.exec_ctx(admin),
                addr.to_string(),
                coin(75_000_000, TEST_DENOM), // same as default
            )
            .unwrap_err();

        assert!(matches!(
            err,
            EcashContractError::ReducedDepositNotReduced { .. }
        ));

        Ok(())
    }

    #[test]
    fn set_reduced_deposit_price_rejects_amount_above_default() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");
        let admin = test.admin_info();

        let err = CONTRACT
            .set_reduced_deposit_price(
                test.exec_ctx(admin),
                addr.to_string(),
                coin(100_000_000, TEST_DENOM),
            )
            .unwrap_err();

        assert!(matches!(
            err,
            EcashContractError::ReducedDepositNotReduced { .. }
        ));

        Ok(())
    }

    #[test]
    fn set_reduced_deposit_price_stores_price() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");
        let reduced = coin(10_000_000, TEST_DENOM);
        let admin = test.admin_info();

        CONTRACT.set_reduced_deposit_price(
            test.exec_ctx(admin),
            addr.to_string(),
            reduced.clone(),
        )?;

        let stored =
            CONTRACT.get_reduced_deposit_amount(test.query_ctx(), addr.to_string())?;

        assert_eq!(stored, Some(reduced));

        Ok(())
    }

    #[test]
    fn set_reduced_deposit_price_overwrites_existing_price() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");

        let admin = test.admin_info();
        CONTRACT.set_reduced_deposit_price(
            test.exec_ctx(admin),
            addr.to_string(),
            coin(10_000_000, TEST_DENOM),
        )?;

        let admin = test.admin_info();
        CONTRACT.set_reduced_deposit_price(
            test.exec_ctx(admin),
            addr.to_string(),
            coin(5_000_000, TEST_DENOM),
        )?;

        let stored =
            CONTRACT.get_reduced_deposit_amount(test.query_ctx(), addr.to_string())?;

        assert_eq!(stored, Some(coin(5_000_000, TEST_DENOM)));

        Ok(())
    }

    // --- remove_reduced_deposit_price ---

    #[test]
    fn remove_reduced_deposit_price_requires_admin() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let non_admin = test.deps.api.addr_make("non_admin");
        let addr = test.deps.api.addr_make("alice");

        let err = CONTRACT
            .remove_reduced_deposit_price(
                test.exec_ctx(message_info(&non_admin, &[])),
                addr.to_string(),
            )
            .unwrap_err();

        assert!(matches!(err, EcashContractError::Admin(_)));

        Ok(())
    }

    #[test]
    fn remove_reduced_deposit_price_clears_stored_price() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");

        let admin = test.admin_info();
        CONTRACT.set_reduced_deposit_price(
            test.exec_ctx(admin),
            addr.to_string(),
            coin(10_000_000, TEST_DENOM),
        )?;

        let admin = test.admin_info();
        CONTRACT.remove_reduced_deposit_price(test.exec_ctx(admin), addr.to_string())?;

        let stored = CONTRACT.get_reduced_deposit_amount(test.query_ctx(), addr.to_string())?;

        assert!(stored.is_none());

        Ok(())
    }

    #[test]
    fn remove_reduced_deposit_price_errors_for_unlisted_address() -> anyhow::Result<()> {
        let mut test = TestSetup::init();
        let addr = test.deps.api.addr_make("alice");

        let admin = test.admin_info();
        let err = CONTRACT
            .remove_reduced_deposit_price(test.exec_ctx(admin), addr.to_string())
            .unwrap_err();

        assert_eq!(
            err,
            EcashContractError::NoReducedDepositPrice {
                address: addr.to_string()
            }
        );

        Ok(())
    }
}
