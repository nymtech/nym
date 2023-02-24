// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_contract_testing::{env_with_block_info, ContractState, SingleContractMock};
use cosmwasm_std::from_slice;
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{BlockInfo, Timestamp};
use mixnet_contract::mixnet_contract_settings::storage::CONTRACT_STATE;
use mixnet_contract::mixnodes::queries::query_mixnode_details;
use mixnet_contract::MixnetContract;
use mixnet_contract::{mixnet_contract_settings, mixnodes};
use mixnet_contract_common::{ContractState as MixnetContractState, ExecuteMsg, Layer, QueryMsg};

const MIXNET_CONTRACT_ADDRESS: &str =
    "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";

fn set_mock() -> SingleContractMock<MixnetContract> {
    let current_block = BlockInfo {
        height: 1928125,
        time: Timestamp::from_seconds(1676482616),
        chain_id: "nymnet".to_string(),
    };
    let custom_env = env_with_block_info(current_block);

    let mix_state = ContractState::try_from_state_dump(
        "../integration-tests/contract-states/15.02.23-173000-qwerty-mixnet.json",
        Some(custom_env.clone()),
    )
    .unwrap()
    .with_contract_address(MIXNET_CONTRACT_ADDRESS);

    SingleContractMock::new(mix_state)
}

fn normal_queries() {
    let mock = set_mock();

    // the simplest example of a query: 'what's the current contract state?'
    let query = QueryMsg::GetState {};
    let result: MixnetContractState = mock.query_de(query).unwrap();
    // println!("{:?}", result);
    assert_eq!(
        "n1fxwdqgwht4j2suv5pr55304kt9z0avrvxs9ls0",
        result.owner.as_ref()
    );
}

fn queries_with_native_functions() {
    let mock = set_mock();

    // access exactly the same information as before, but this time with native functions
    let deps = mock.deps();
    let result = mixnet_contract_settings::queries::query_contract_state(deps).unwrap();
    // println!("{:?}", result);
    assert_eq!(
        "n1fxwdqgwht4j2suv5pr55304kt9z0avrvxs9ls0",
        result.owner.as_ref()
    );
}

fn raw_storage_reads() {
    // we can also read any arbitrary data that's normally not exposed via queries
    // for this example, let's read exactly the same data again
    let mock = set_mock();

    // wrapped in a cw-storage-plus 'item'
    let result = mock.state.load_item(&CONTRACT_STATE).unwrap();
    // println!("{:?}", result);
    assert_eq!(
        "n1fxwdqgwht4j2suv5pr55304kt9z0avrvxs9ls0",
        result.owner.as_ref()
    );

    // a raw key-value read
    let result_raw = mock.state.read_key(b"state").unwrap();
    let result: MixnetContractState = from_slice(&result_raw).unwrap();
    assert_eq!(
        "n1fxwdqgwht4j2suv5pr55304kt9z0avrvxs9ls0",
        result.owner.as_ref()
    );
}

fn normal_transactions() {
    let mut mock = set_mock();

    // pretend you're the rewarding validator and force assign somebody's layer!
    let current_mixnode = query_mixnode_details(mock.deps(), 7).unwrap();
    assert_eq!(
        current_mixnode
            .mixnode_details
            .unwrap()
            .bond_information
            .layer,
        Layer::One
    );
    let rewarding_validator = mixnet_contract_settings::queries::query_contract_state(mock.deps())
        .unwrap()
        .rewarding_validator_address;

    let msg_sender = mock_info(rewarding_validator.as_ref(), &[]);
    let msg = ExecuteMsg::AssignNodeLayer {
        mix_id: 7,
        layer: Layer::Two,
    };
    mock.execute(msg_sender, msg).unwrap();

    let updated_mixnode = query_mixnode_details(mock.deps(), 7).unwrap();
    assert_eq!(
        updated_mixnode
            .mixnode_details
            .unwrap()
            .bond_information
            .layer,
        Layer::Two
    );
}

fn changing_state_with_native_functions() {
    // do the same thing but this time calling contract methods directly
    let mut mock = set_mock();

    let current_mixnode = query_mixnode_details(mock.deps(), 7).unwrap();
    assert_eq!(
        current_mixnode
            .mixnode_details
            .unwrap()
            .bond_information
            .layer,
        Layer::One
    );
    let rewarding_validator = mixnet_contract_settings::queries::query_contract_state(mock.deps())
        .unwrap()
        .rewarding_validator_address;

    let msg_sender = mock_info(rewarding_validator.as_ref(), &[]);
    let deps = mock.deps_mut();

    mixnodes::transactions::assign_mixnode_layer(deps, msg_sender, 7, Layer::Two).unwrap();
    let updated_mixnode = query_mixnode_details(mock.deps(), 7).unwrap();
    assert_eq!(
        updated_mixnode
            .mixnode_details
            .unwrap()
            .bond_information
            .layer,
        Layer::Two
    );
}

fn writing_to_raw_storage() {
    // bypass this whole transaction business, authorization checks, etc and just write to the storage yourself
    let mut mock = set_mock();

    let mut mix_bond = mixnodes::storage::mixnode_bonds()
        .load(mock.deps().storage, 7)
        .unwrap();
    assert_eq!(mix_bond.layer, Layer::One);
    mix_bond.layer = Layer::Two;

    mixnodes::storage::mixnode_bonds()
        .save(mock.deps_mut().storage, 7, &mix_bond)
        .unwrap();

    let updated_mixnode = query_mixnode_details(mock.deps(), 7).unwrap();
    assert_eq!(
        updated_mixnode
            .mixnode_details
            .unwrap()
            .bond_information
            .layer,
        Layer::Two
    );
}

fn main() {
    normal_queries();
    queries_with_native_functions();
    raw_storage_reads();

    normal_transactions();
    changing_state_with_native_functions();
    writing_to_raw_storage();
}
