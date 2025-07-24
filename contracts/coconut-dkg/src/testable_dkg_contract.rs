// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]

use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use cw4::Member;
use nym_contracts_common_testing::{
    CommonStorageKeys, ContractFn, ContractTester, ContractTesterBuilder, PermissionedFn, QueryFn,
    TEST_DENOM,
};

pub use cw3_flex_multisig::testable_cw3_contract::{Duration, MultisigContract, Threshold};
pub use cw4_group::testable_cw4_contract::GroupContract;
pub use nym_coconut_dkg_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
pub use nym_contracts_common_testing::TestableNymContract;

pub struct DkgContract;

const DEFAULT_GROUP_MEMBERS: usize = 15;

impl TestableNymContract for DkgContract {
    const NAME: &'static str = "dkg-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = ContractError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        instantiate
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        execute
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        query
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        migrate
    }

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        init_contract_tester_with_group_members(DEFAULT_GROUP_MEMBERS)
    }
}

pub fn init_contract_tester() -> ContractTester<DkgContract> {
    DkgContract::init().with_common_storage_key(CommonStorageKeys::Admin, "dkg-admin")
}

pub fn prepare_contract_tester_builder_with_group_members<C>(
    members: usize,
) -> ContractTesterBuilder<C>
where
    C: TestableNymContract,
{
    let mut builder = ContractTesterBuilder::<C>::new();
    let api = builder.api();

    // 1. init the CW4 group contract
    let group_init_msg = cw4_group::testable_cw4_contract::InstantiateMsg {
        admin: Some(api.addr_make("group-admin").to_string()),
        members: (0..members)
            .map(|i| Member {
                addr: api.addr_make(&format!("group-member-{i}")).to_string(),
                weight: 1,
            })
            .collect(),
    };
    builder.instantiate_contract::<GroupContract>(Some(group_init_msg));

    // we just instantiated it
    let group_contract_address = builder.unchecked_contract_address::<GroupContract>();

    // 2. init the CW3 multisig contract WITH DUMMY VALUES
    let multisig_init_msg = cw3_flex_multisig::testable_cw3_contract::InstantiateMsg {
        group_addr: group_contract_address.to_string(),
        // \/ PLACEHOLDERS
        coconut_bandwidth_contract_address: group_contract_address.to_string(),
        coconut_dkg_contract_address: group_contract_address.to_string(),
        // /\ PLACEHOLDERS
        threshold: Threshold::AbsolutePercentage {
            percentage: "0.67".parse().unwrap(),
        },
        max_voting_period: Duration::Time(3600),
        executor: None,
        proposal_deposit: None,
    };
    builder.instantiate_contract::<MultisigContract>(Some(multisig_init_msg));

    // we just instantiated it
    let multisig_contract_address = builder.unchecked_contract_address::<MultisigContract>();

    // 3. init the DKG contract
    let dkg_init_msg = InstantiateMsg {
        group_addr: group_contract_address.to_string(),
        multisig_addr: multisig_contract_address.to_string(),
        time_configuration: None,
        mix_denom: TEST_DENOM.to_string(),
        key_size: 5,
    };
    builder.instantiate_contract::<DkgContract>(Some(dkg_init_msg));

    // we just instantiated it
    let dkg_contract_address = builder.unchecked_contract_address::<DkgContract>();

    // 4. migrate the multisig contract to hold correct addresses
    let multisig_migrate_msg = cw3_flex_multisig::testable_cw3_contract::MigrateMsg {
        // \/ STILL A PLACEHOLDER (this contract does not care about interactions with the ecash contract)
        coconut_bandwidth_address: dkg_contract_address.to_string(),
        // /\ STILL A PLACEHOLDER
        coconut_dkg_address: dkg_contract_address.to_string(),
    };
    builder.migrate_contract::<MultisigContract>(&multisig_migrate_msg);
    builder
}

pub fn init_contract_tester_with_group_members(members: usize) -> ContractTester<DkgContract> {
    prepare_contract_tester_builder_with_group_members(members).build()
}
