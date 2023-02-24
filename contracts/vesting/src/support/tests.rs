#[cfg(test)]
pub mod helpers {
    use crate::contract::{instantiate, try_create_periodic_vesting_account};
    use crate::storage::{ACCOUNTS, ADMIN, MIXNET_CONTRACT_ADDRESS, MIX_DENOM};
    use crate::traits::VestingAccount;
    use crate::vesting::{populate_vesting_periods, Account};
    use contracts_common::Percent;
    use cosmwasm_contract_testing::{env_with_block_info, ContractState};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{
        coin, Addr, BlockInfo, Coin, Deps, DepsMut, Empty, Env, MemoryStorage, MessageInfo,
        OwnedDeps, Storage, Timestamp, Uint128,
    };
    use std::path::Path;
    use std::str::FromStr;
    use vesting_contract_common::messages::{InitMsg, VestingSpecification};
    use vesting_contract_common::PledgeCap;

    #[allow(unused)]
    pub struct TestSetup {
        pub state: ContractState,
        pub admin: MessageInfo,
    }

    #[allow(unused)]
    impl TestSetup {
        pub fn new() -> Self {
            let deps = init_contract();
            let admin = ADMIN.load(deps.as_ref().storage).unwrap();

            TestSetup {
                state: ContractState::new(),
                admin: mock_info(admin.as_str(), &[]),
            }
        }

        pub fn new_from_state_dump<P: AsRef<Path>>(dump_file: P) -> Self {
            let current_block = BlockInfo {
                height: 5633424,
                time: Timestamp::from_seconds(1676025955),
                chain_id: "nyx".to_string(),
            };
            let custom_env = env_with_block_info(current_block);
            let state = ContractState::try_from_state_dump(dump_file, Some(custom_env.clone()))
                .unwrap()
                .with_contract_address(
                    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw",
                );

            let admin = ADMIN.load(state.deps().storage).unwrap();

            TestSetup {
                state,
                admin: mock_info(admin.as_str(), &[]),
            }
        }

        pub fn create_vesting_account(
            &mut self,
            owner: &str,
            spec: Option<VestingSpecification>,
            amount: u128,
        ) -> Account {
            let mut sender = self.admin();
            sender.funds = vec![self.coin(amount)];
            let env = self.env();
            try_create_periodic_vesting_account(
                owner,
                None,
                spec,
                Some(PledgeCap::Percent(
                    Percent::from_percentage_value(100).unwrap(),
                )),
                sender,
                env,
                self.deps_mut(),
            )
            .unwrap();

            self.unchecked_account(Addr::unchecked(owner))
        }

        pub fn deps(&self) -> Deps<'_> {
            self.state.deps()
        }

        pub fn deps_mut(&mut self) -> DepsMut<'_> {
            self.state.deps_mut()
        }

        pub fn env(&self) -> Env {
            self.state.env_cloned()
        }

        pub fn admin(&self) -> MessageInfo {
            self.admin.clone()
        }

        pub fn mixnet_contract(&self) -> Addr {
            MIXNET_CONTRACT_ADDRESS.load(self.deps().storage).unwrap()
        }

        pub fn coin(&self, amount: u128) -> Coin {
            let denom = MIX_DENOM.load(self.deps().storage).unwrap();
            coin(amount, denom)
        }

        pub fn unchecked_account(&self, address: Addr) -> Account {
            ACCOUNTS.load(self.deps().storage, address).unwrap()
        }

        pub fn print_account_coins(&self, account: &Account) {
            let original = self.original_vesting(account);
            let balance = self.balance(account);
            let withdrawn = self.withdrawn(account);
            let vesting = self.vesting_coins(account);
            let vested = self.vested_coins(account);

            let locked = self.locked_coins(account);
            let spendable = self.spendable_coins(account);
            let spendable_vested = self.spendable_vested_coins(account);
            let spendable_reward = self.spendable_reward_coins(account);

            assert_eq!(
                spendable.amount,
                spendable_vested.amount + spendable_reward.amount
            );

            let total_delegated = self.delegated_total(account);
            let historical_rewards = self.historical_staking_rewards(account);

            let pretty = format!(
                r#"
        {:<20}{original}
        {:<20}{vesting}
        {:<20}{vested}
        {:<20}{balance}
        {:<20}{withdrawn}
        {:<20}{historical_rewards}
        {:<20}{locked}
        {:<20}{spendable}
        {:<20}{spendable_vested}
        {:<20}{spendable_reward}
        {:<20}{total_delegated}
        "#,
                "original",
                "vesting",
                "vested:",
                "balance",
                "withdrawn",
                "historical rewards",
                "locked",
                "spendable",
                "spendable vested",
                "spendable reward",
                "total delegated",
            );

            println!("{pretty}")
        }

        pub fn original_vesting(&self, account: &Account) -> Coin {
            account.get_original_vesting().unwrap().amount()
        }

        pub fn balance(&self, account: &Account) -> Coin {
            self.coin(account.load_balance(self.deps().storage).unwrap().u128())
        }

        pub fn withdrawn(&self, account: &Account) -> Coin {
            self.coin(account.load_withdrawn(self.deps().storage).unwrap().u128())
        }

        pub fn vesting_coins(&self, account: &Account) -> Coin {
            account
                .get_vesting_coins(None, self.state.env(), self.deps().storage)
                .unwrap()
        }

        pub fn vested_coins(&self, account: &Account) -> Coin {
            account
                .get_vested_coins(None, self.state.env(), self.deps().storage)
                .unwrap()
        }

        pub fn locked_coins(&self, account: &Account) -> Coin {
            account
                .locked_coins(None, self.state.env(), self.deps().storage)
                .unwrap()
        }

        pub fn spendable_coins(&self, account: &Account) -> Coin {
            account
                .spendable_coins(None, self.state.env(), self.deps().storage)
                .unwrap()
        }

        pub fn spendable_vested_coins(&self, account: &Account) -> Coin {
            account
                .spendable_vested_coins(None, self.state.env(), self.deps().storage)
                .unwrap()
        }

        pub fn spendable_reward_coins(&self, account: &Account) -> Coin {
            account
                .spendable_reward_coins(None, self.state.env(), self.deps().storage)
                .unwrap()
        }

        pub fn delegated_total(&self, account: &Account) -> Coin {
            self.coin(
                account
                    .total_delegations(self.deps().storage)
                    .unwrap()
                    .u128(),
            )
        }

        pub fn historical_staking_rewards(&self, account: &Account) -> Coin {
            account
                .get_historical_vested_staking_rewards(self.deps().storage)
                .unwrap()
        }
    }

    pub const TEST_COIN_DENOM: &str = "unym";

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InitMsg {
            mixnet_contract_address: "test".to_string(),
            mix_denom: TEST_COIN_DENOM.to_string(),
        };
        let env = mock_env();
        let info = mock_info("admin", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        deps
    }

    pub fn vesting_account_mid_fixture(storage: &mut dyn Storage, env: &Env) -> Account {
        let start_time_ts = env.block.time.clone();
        let start_time = env.block.time.seconds() - 7200;
        let periods = populate_vesting_periods(
            start_time,
            VestingSpecification::new(None, Some(3600), None),
        );

        Account::new(
            Addr::unchecked("owner"),
            Some(Addr::unchecked("staking")),
            Coin {
                amount: Uint128::new(1_000_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            start_time_ts,
            periods,
            None,
            storage,
        )
        .unwrap()
    }

    pub fn vesting_account_new_fixture(storage: &mut dyn Storage, env: &Env) -> Account {
        let start_time = env.block.time;
        let periods =
            populate_vesting_periods(start_time.seconds(), VestingSpecification::default());

        Account::new(
            Addr::unchecked("owner"),
            Some(Addr::unchecked("staking")),
            Coin {
                amount: Uint128::new(1_000_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            start_time,
            periods,
            Some(PledgeCap::from_str("0.1").unwrap()),
            storage,
        )
        .unwrap()
    }

    pub fn vesting_account_percent_fixture(storage: &mut dyn Storage, env: &Env) -> Account {
        let start_time = env.block.time;
        let periods =
            populate_vesting_periods(start_time.seconds(), VestingSpecification::default());

        Account::new(
            Addr::unchecked("owner"),
            Some(Addr::unchecked("staking")),
            Coin {
                amount: Uint128::new(1_000_000_000_000),
                denom: TEST_COIN_DENOM.to_string(),
            },
            start_time,
            periods,
            Some(PledgeCap::Percent(
                Percent::from_percentage_value(10).unwrap(),
            )),
            storage,
        )
        .unwrap()
    }
}
