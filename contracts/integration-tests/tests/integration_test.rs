// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_contract_testing::{
    env_with_block_info, ContractState, MultiContractMock, TestableContract,
};
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    Addr, BankMsg, BlockInfo, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, Timestamp,
};
use cw_storage_plus::Map;
use nym_mixnet_contract_common::rewarding::PendingRewardResponse;
use vesting_contract::vesting::Account;

struct VestingContract;

impl TestableContract for VestingContract {
    type ContractError = vesting_contract::errors::ContractError;
    type InstantiateMsg = nym_vesting_contract_common::InitMsg;
    type ExecuteMsg = nym_vesting_contract_common::ExecuteMsg;
    type QueryMsg = nym_vesting_contract_common::QueryMsg;

    fn new() -> Self {
        VestingContract
    }

    fn instantiate(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::InstantiateMsg,
    ) -> Result<Response, Self::ContractError> {
        vesting_contract::contract::instantiate(deps, env, info, msg)
    }

    fn execute(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::ExecuteMsg,
    ) -> Result<Response, Self::ContractError> {
        vesting_contract::contract::execute(deps, env, info, msg)
    }

    fn query(
        deps: Deps<'_>,
        env: Env,
        msg: Self::QueryMsg,
    ) -> Result<QueryResponse, Self::ContractError> {
        vesting_contract::contract::query(deps, env, msg)
    }
}

struct MixnetContract;

impl TestableContract for MixnetContract {
    type ContractError = nym_mixnet_contract_common::error::MixnetContractError;
    type InstantiateMsg = nym_mixnet_contract_common::InstantiateMsg;
    type ExecuteMsg = nym_mixnet_contract_common::ExecuteMsg;
    type QueryMsg = nym_mixnet_contract_common::QueryMsg;

    fn new() -> Self {
        MixnetContract
    }

    fn instantiate(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::InstantiateMsg,
    ) -> Result<Response, Self::ContractError> {
        mixnet_contract::contract::instantiate(deps, env, info, msg)
    }

    fn execute(
        deps: DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: Self::ExecuteMsg,
    ) -> Result<Response, Self::ContractError> {
        mixnet_contract::contract::execute(deps, env, info, msg)
    }

    fn query(
        deps: Deps<'_>,
        env: Env,
        msg: Self::QueryMsg,
    ) -> Result<QueryResponse, Self::ContractError> {
        mixnet_contract::contract::query(deps, env, msg)
    }
}

// this is not directly exported by the vesting contract, but we can easily recreate it
const VESTING_ACCOUNTS: Map<'_, Addr, Account> = Map::new("acc");

const MIXNET_CONTRACT_ADDRESS: &str =
    "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";

fn set_mock() -> MultiContractMock {
    let current_block = BlockInfo {
        height: 1928125,
        time: Timestamp::from_seconds(1676482616),
        chain_id: "nymnet".to_string(),
    };
    let custom_env = env_with_block_info(current_block);

    let mix_mock = ContractState::try_from_state_dump(
        "contract-states/15.02.23-173000-qwerty-mixnet.json",
        Some(custom_env.clone()),
    )
    .unwrap()
    .with_contract_address(MIXNET_CONTRACT_ADDRESS);
    let vesting_mock = ContractState::try_from_state_dump(
        "contract-states/15.02.23-173000-qwerty-vesting.json",
        Some(custom_env),
    )
    .unwrap()
    .with_contract_address(VESTING_CONTRACT_ADDRESS);

    let mut multi_mock = MultiContractMock::new();

    multi_mock.add_contract::<MixnetContract>(mix_mock).unwrap();
    multi_mock
        .add_contract::<VestingContract>(vesting_mock)
        .unwrap();
    multi_mock
}

#[test]
fn claiming_vesting_delegator_rewards() {
    let mut multi_mock = set_mock();

    let dummy_account = Addr::unchecked("n1ktpuwtweku40uaxcl4uq7mdkkmjeh698g3l3c8");

    // do some queries to verify state is updated correctly for both contracts
    let pending_reward: PendingRewardResponse = multi_mock
        .query::<MixnetContract, _>(
            MIXNET_CONTRACT_ADDRESS,
            nym_mixnet_contract_common::QueryMsg::GetPendingDelegatorReward {
                address: dummy_account.to_string(),
                mix_id: 8,
                proxy: Some(VESTING_CONTRACT_ADDRESS.to_string()),
            },
        )
        .unwrap();
    let pending_reward_amount = pending_reward.amount_earned.unwrap().amount;

    // we can also get whatever we want directly from storage!
    let contract_state = multi_mock.contract_state(VESTING_CONTRACT_ADDRESS).unwrap();
    let vesting_account = contract_state
        .load_map_value(&VESTING_ACCOUNTS, dummy_account.clone())
        .unwrap();
    let vesting_balance = vesting_account
        .load_balance(contract_state.deps().storage)
        .unwrap();

    let res = multi_mock.execute_full::<VestingContract>(
        VESTING_CONTRACT_ADDRESS,
        mock_info(dummy_account.as_str(), &[]),
        nym_vesting_contract_common::ExecuteMsg::ClaimDelegatorReward { mix_id: 8 },
    );

    match res {
        Ok(success) => {
            println!("{}", success.pretty());

            // check the output

            // unfortunately `ClaimDelegatorReward` doesn't emit any events, but we can see
            // it's going to result into a call into the mixnet contract
            assert_eq!(
                success.steps[0].further_execution[0].contract.as_str(),
                MIXNET_CONTRACT_ADDRESS
            );

            // mixnet contract will emit a `v2_withdraw_delegator_reward` event
            // and call the vesting contract again
            assert_eq!(
                "v2_withdraw_delegator_reward",
                success.steps[1].events[0].ty
            );
            assert_eq!(
                success.steps[1].further_execution[0].contract.as_str(),
                VESTING_CONTRACT_ADDRESS
            );
            // and will move our reward amount into the vesting contract...
            assert!(matches!(
                &success.steps[1].bank_msgs[0],
                BankMsg::Send { to_address, amount }
                if to_address == VESTING_CONTRACT_ADDRESS && amount[0].amount == pending_reward_amount
            ));

            // and finally the vesting contract will emit the mistyped `track_reaward` event
            assert_eq!("track_reaward", success.steps[2].events[0].ty);
        }
        Err(err) => panic!("{err}"),
    }

    // state after execution (we can still read values the 'normal' way)
    let updated_state = multi_mock.contract_state(VESTING_CONTRACT_ADDRESS).unwrap();
    let deps = updated_state.deps();
    let vesting_account = VESTING_ACCOUNTS.load(deps.storage, dummy_account).unwrap();
    let new_vesting_balance = vesting_account.load_balance(deps.storage).unwrap();
    assert_eq!(new_vesting_balance, vesting_balance + pending_reward_amount)
}
