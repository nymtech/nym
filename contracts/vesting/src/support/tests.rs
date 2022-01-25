#[cfg(test)]
pub mod helpers {
    use crate::contract::{instantiate, NUM_VESTING_PERIODS};
    use crate::messages::InitMsg;
    use crate::vesting::{populate_vesting_periods, Account};
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{Addr, Coin, Empty, Env, MemoryStorage, OwnedDeps, Storage, Uint128};

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InitMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }

    pub fn vesting_account_fixture(storage: &mut dyn Storage, env: &Env) -> Account {
        let start_time = env.block.time;
        let periods = populate_vesting_periods(start_time.seconds(), NUM_VESTING_PERIODS);

        Account::new(
            Addr::unchecked("owner"),
            Some(Addr::unchecked("staking")),
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
