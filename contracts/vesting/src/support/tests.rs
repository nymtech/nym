#[cfg(test)]
pub mod helpers {
    use crate::contract::{instantiate, NUM_VESTING_PERIODS, VESTING_PERIOD};
    use crate::messages::InitMsg;
    use crate::storage;
    use crate::vesting::populate_vesting_periods;
    use crate::vesting::PeriodicVestingAccount;
    use crate::vesting::VestingPeriod;
    use config::defaults::DENOM;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::Addr;
    use cosmwasm_std::Coin;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::Uint128;
    use cosmwasm_std::{Empty, Env, MemoryStorage, Storage};

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InitMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }

    pub fn vesting_account_fixture(storage: &mut dyn Storage, env: &Env) -> PeriodicVestingAccount {
        let start_time = env.block.time;
        let periods = populate_vesting_periods(start_time.seconds(), NUM_VESTING_PERIODS);

        PeriodicVestingAccount::new(
            Addr::unchecked("fixture"),
            Coin {
                amount: Uint128::new(1_000_000_000_000),
                denom: DENOM.to_string(),
            },
            start_time,
            periods,
            storage,
        )
        .unwrap()
    }
}
