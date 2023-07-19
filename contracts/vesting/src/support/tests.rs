#[cfg(test)]
pub mod helpers {

    // TODO: once https://github.com/nymtech/nym/pull/3040 gets merged,
    // the `ContractState` should replace the below
    #[allow(unused)]
    mod state_dump_decoder {
        use base64::{engine::general_purpose, Engine};
        use serde::{Deserialize, Serialize};
        use std::fs::File;
        use std::path::Path;

        #[derive(Serialize, Deserialize, Debug)]
        pub struct RawState {
            pub height: String,
            pub result: Vec<RawKV>,
        }

        impl RawState {
            pub fn decode(self) -> DecodedState {
                DecodedState {
                    height: self.height.parse().unwrap(),
                    result: self
                        .result
                        .into_iter()
                        .map(|raw| DecodedKV {
                            key: hex::decode(&raw.key).unwrap(),
                            value: general_purpose::STANDARD.decode(&raw.value).unwrap(),
                        })
                        .collect(),
                }
            }

            pub fn from_file<P: AsRef<Path>>(path: P) -> Self {
                let file = File::open(path).expect("failed to open specified file");
                serde_json::from_reader(file).expect("failed to parse specified file")
            }
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct RawKV {
            // hex
            pub key: String,

            // base64
            pub value: String,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct DecodedKV {
            pub key: Vec<u8>,
            pub value: Vec<u8>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct DecodedState {
            pub height: u64,
            pub result: Vec<DecodedKV>,
        }

        impl DecodedState {
            pub fn find_value(&self, key: &[u8]) -> Option<Vec<u8>> {
                self.result
                    .iter()
                    .find(|kv| kv.key == key)
                    .map(|kv| kv.value.clone())
            }
        }
    }

    use crate::contract::{instantiate, try_create_periodic_vesting_account};
    use crate::storage::{ACCOUNTS, ADMIN, MIXNET_CONTRACT_ADDRESS, MIX_DENOM};
    use crate::support::tests::helpers::state_dump_decoder::RawState;
    use crate::traits::VestingAccount;
    use crate::vesting::{populate_vesting_periods, StorableVestingAccountExt};
    use contracts_common::Percent;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{
        coin, Addr, BlockInfo, Coin, ContractInfo, Deps, DepsMut, Empty, Env, MemoryStorage,
        MessageInfo, OwnedDeps, Storage, Timestamp, Uint128,
    };
    use rand_chacha::rand_core::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use std::path::Path;
    use std::str::FromStr;
    use vesting_contract_common::messages::InitMsg;
    use vesting_contract_common::{Account, PledgeCap, VestingSpecification};

    // use rng with constant seed for all tests so that they would be deterministic
    #[allow(unused)]
    pub fn test_rng() -> ChaCha20Rng {
        let dummy_seed = [42u8; 32];
        rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
    }

    #[allow(unused)]
    pub struct TestSetup {
        pub deps: OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
        pub env: Env,
        pub rng: ChaCha20Rng,

        pub admin: MessageInfo,
    }

    #[allow(unused)]
    impl TestSetup {
        pub fn new() -> Self {
            let deps = init_contract();
            let admin = ADMIN.load(deps.as_ref().storage).unwrap();

            TestSetup {
                deps,
                env: mock_env(),
                rng: test_rng(),
                admin: mock_info(admin.as_str(), &[]),
            }
        }

        pub fn new_from_state_dump<P: AsRef<Path>>(dump_file: P) -> Self {
            let state = RawState::from_file(dump_file).decode();

            let mut deps = mock_dependencies();
            for kv in state.result {
                deps.storage.set(&kv.key, &kv.value)
            }

            let admin = ADMIN.load(deps.as_ref().storage).unwrap();
            let env = Env {
                block: BlockInfo {
                    height: 5633424,
                    time: Timestamp::from_seconds(1676025955),
                    chain_id: "nyx".to_string(),
                },
                transaction: None,
                contract: ContractInfo {
                    address: Addr::unchecked(
                        "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw",
                    ),
                },
            };

            TestSetup {
                deps,
                env,
                rng: test_rng(),
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
            self.deps.as_ref()
        }

        pub fn deps_mut(&mut self) -> DepsMut<'_> {
            self.deps.as_mut()
        }

        pub fn env(&self) -> Env {
            self.env.clone()
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
                .get_vesting_coins(None, &self.env, self.deps().storage)
                .unwrap()
        }

        pub fn vested_coins(&self, account: &Account) -> Coin {
            account
                .get_vested_coins(None, &self.env, self.deps().storage)
                .unwrap()
        }

        pub fn locked_coins(&self, account: &Account) -> Coin {
            account
                .locked_coins(None, &self.env, self.deps().storage)
                .unwrap()
        }

        pub fn spendable_coins(&self, account: &Account) -> Coin {
            account
                .spendable_coins(None, &self.env, self.deps().storage)
                .unwrap()
        }

        pub fn spendable_vested_coins(&self, account: &Account) -> Coin {
            account
                .spendable_vested_coins(None, &self.env, self.deps().storage)
                .unwrap()
        }

        pub fn spendable_reward_coins(&self, account: &Account) -> Coin {
            account
                .spendable_reward_coins(None, &self.env, self.deps().storage)
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
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }

    pub fn vesting_account_mid_fixture(storage: &mut dyn Storage, env: &Env) -> Account {
        let start_time_ts = env.block.time;
        let start_time = env.block.time.seconds() - 7200;
        let periods = populate_vesting_periods(
            start_time,
            VestingSpecification::new(None, Some(3600), None),
        );

        Account::save_new(
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

        Account::save_new(
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

        Account::save_new(
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
