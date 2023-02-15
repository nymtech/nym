// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_contract_testing::{
    env_with_block_info, ContractMock, MultiContractMock, TestableContract,
};
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    coin, BlockInfo, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, Timestamp,
};

struct VestingContract;

impl TestableContract for VestingContract {
    type ContractError = vesting_contract::errors::ContractError;
    type ExecuteMsg = vesting_contract_common::ExecuteMsg;
    type QueryMsg = vesting_contract_common::QueryMsg;

    fn new() -> Self {
        VestingContract
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
    type ContractError = mixnet_contract_common::error::MixnetContractError;
    type ExecuteMsg = mixnet_contract_common::ExecuteMsg;
    type QueryMsg = mixnet_contract_common::QueryMsg;

    fn new() -> Self {
        MixnetContract
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

#[test]
fn multi_mock() {
    let mixnet_contract_address = "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
    let vesting_contract_address = "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";

    let current_block = BlockInfo {
        height: 1928125,
        time: Timestamp::from_seconds(1676482616),
        chain_id: "nymnet".to_string(),
    };
    let custom_env = env_with_block_info(current_block);

    let mix_mock = ContractMock::try_from_state_dump(
        "contract-states/15.02.23-173000-qwerty-mixnet.json",
        Some(custom_env.clone()),
    )
    .unwrap()
    .with_contract_address(mixnet_contract_address);
    let vesting_mock = ContractMock::try_from_state_dump(
        "contract-states/15.02.23-173000-qwerty-vesting.json",
        Some(custom_env),
    )
    .unwrap()
    .with_contract_address(vesting_contract_address);

    let mut multi_mock = MultiContractMock::new();

    multi_mock.add_contract::<MixnetContract>(mix_mock).unwrap();
    multi_mock
        .add_contract::<VestingContract>(vesting_mock)
        .unwrap();

    let res = multi_mock.execute_full::<VestingContract>(
        vesting_contract_address,
        mock_info("n1vuz06p7cgagxcaplfezchvpu99u4np7erfxa4c", &[]),
        vesting_contract_common::ExecuteMsg::DelegateToMixnode {
            mix_id: 7,
            amount: coin(1000, "unym"),
            on_behalf_of: None,
        },
    );

    match res {
        Ok(success) => {
            // first we should have emitted a "vesting_delegation" event from the vesting contract
            // followed by "v2_pending_delegation" from the mixnet contract
            assert_eq!("vesting_delegation", success.steps[0].events[0].ty);
            assert_eq!("v2_pending_delegation", success.steps[1].events[0].ty);

            // println!("{}", success.pretty())
        }
        Err(err) => panic!("{err}"),
    }
}
