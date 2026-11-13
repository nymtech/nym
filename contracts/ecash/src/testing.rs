// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::contract::{execute, instantiate, migrate, query, reply};
use crate::storage::ECASH_CONTRACT_STORAGE;
use crate::transactions::{try_deposit_ticket_book_funds, try_set_reduced_deposit_price};
use cosmwasm_std::testing::message_info;
use cosmwasm_std::{coin, Addr, Empty, Response, StdError};
use cw_multi_test::{Contract, ContractWrapper};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageWriter, CommonStorageKeys, ContractFn, ContractOpts,
    ContractTester, ContractTesterBuilder, PermissionedFn, QueryFn, TestableNymContract,
    TEST_DENOM,
};
use nym_ecash_contract_common::deposit::DepositId;
use nym_ecash_contract_common::events::PROPOSAL_ID_ATTRIBUTE_NAME;
use nym_ecash_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use nym_ecash_contract_common::EcashContractError;
use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;

pub(crate) const DEFAULT_DEPOSIT_AMOUNT: u128 = 75_000_000;

/// A valid bs58-encoded ed25519 public key for deposits.
pub(crate) const DUMMY_IDENTITY_KEY: &str = "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK";

/// Proposal id the stub multisig reports for every `Propose`.
pub(crate) const STUB_PROPOSAL_ID: u64 = 42;

/// Key in the stub multisig's storage that makes every execute fail once set.
const STUB_FAIL_KEY: &[u8] = b"fail";

pub(crate) struct EcashContract;

impl TestableNymContract for EcashContract {
    const NAME: &'static str = "ecash-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = EcashContractError;

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

    // the only contract with a reply handler, so it wires it up itself
    fn dyn_contract() -> Box<dyn Contract<Empty>> {
        Box::new(
            ContractWrapper::new(execute, instantiate, query)
                .with_migrate(migrate)
                .with_reply(reply),
        )
    }

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        let builder = ContractTesterBuilder::new().instantiate::<StubMultisig>(None);

        // we just instantiated it
        let multisig_address = builder
            .well_known_contracts
            .get(StubMultisig::NAME)
            .unwrap()
            .clone();
        let api = builder.api();

        builder
            .instantiate::<Self>(Some(InstantiateMsg {
                holding_account: api.addr_make("holding").to_string(),
                multisig_addr: multisig_address.to_string(),
                group_addr: api.addr_make("group").to_string(),
                deposit_amount: coin(DEFAULT_DEPOSIT_AMOUNT, TEST_DENOM),
            }))
            .build()
    }
}

/// Stand-in for the cw3 multisig: answers every execute with the `proposal_id`
/// attribute the ecash reply handler reads, or fails once [`STUB_FAIL_KEY`] is
/// set in its storage.
pub(crate) struct StubMultisig;

impl TestableNymContract for StubMultisig {
    const NAME: &'static str = "stub-multisig";
    type InitMsg = Empty;
    type ExecuteMsg = MultisigExecuteMsg;
    type QueryMsg = Empty;
    type MigrateMsg = Empty;
    type ContractError = StdError;

    fn instantiate() -> ContractFn<Self::InitMsg, Self::ContractError> {
        |_, _, _, _| Ok(Response::new())
    }

    fn execute() -> ContractFn<Self::ExecuteMsg, Self::ContractError> {
        |deps, _, _, _| {
            if deps.storage.get(STUB_FAIL_KEY).is_some() {
                return Err(StdError::generic_err("stub multisig failure"));
            }
            Ok(Response::new()
                .add_attribute(PROPOSAL_ID_ATTRIBUTE_NAME, STUB_PROPOSAL_ID.to_string()))
        }
    }

    fn query() -> QueryFn<Self::QueryMsg, Self::ContractError> {
        |_, _, _| {
            Err(StdError::generic_err(
                "the stub multisig answers no queries",
            ))
        }
    }

    fn migrate() -> PermissionedFn<Self::MigrateMsg, Self::ContractError> {
        |_, _, _| Ok(Response::new())
    }

    fn base_init_msg() -> Self::InitMsg {
        Empty {}
    }
}

pub(crate) fn init_contract_tester() -> ContractTester<EcashContract> {
    // `cw_controllers::Admin` key of the contract admin in `EcashContractStorage`
    EcashContract::init().with_common_storage_key(CommonStorageKeys::Admin, "contract_admin")
}

pub(crate) trait EcashContractTesterExt:
    ContractOpts<ExecuteMsg = ExecuteMsg, QueryMsg = QueryMsg, ContractError = EcashContractError>
    + AdminExt
    + ArbitraryContractStorageWriter
{
    #[track_caller]
    fn set_reduced_price(&mut self, address: &Addr, amount: u128) {
        let admin = self.admin_msg();
        try_set_reduced_deposit_price(
            self.deps_mut(),
            admin,
            address.to_string(),
            coin(amount, TEST_DENOM),
        )
        .unwrap();
    }

    #[track_caller]
    fn make_deposit(&mut self, depositor: &Addr, amount: u128) {
        let info = message_info(depositor, &[coin(amount, TEST_DENOM)]);
        try_deposit_ticket_book_funds(self.deps_mut(), info, DUMMY_IDENTITY_KEY.to_string())
            .unwrap();
    }

    /// Writes a deposit straight to storage, bypassing the handler.
    #[track_caller]
    fn add_dummy_deposit(&mut self) -> DepositId {
        ECASH_CONTRACT_STORAGE
            .deposits
            .save_deposit(self.storage_mut(), DUMMY_IDENTITY_KEY.to_string())
            .unwrap()
    }

    fn fail_multisig_proposals(&mut self) {
        let multisig = self.unchecked_contract_address::<StubMultisig>();
        ArbitraryContractStorageWriter::set_contract_storage(
            self,
            multisig,
            STUB_FAIL_KEY,
            b"true",
        );
    }
}

impl EcashContractTesterExt for ContractTester<EcashContract> {}
