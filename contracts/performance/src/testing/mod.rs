// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use crate::helpers::MixnetContractQuerier;
use crate::storage::{MeasurementKind, NYM_PERFORMANCE_CONTRACT_STORAGE};
use cosmwasm_std::testing::{MockApi, message_info, mock_env};
use cosmwasm_std::{
    Addr, ContractInfo, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, StdError, StdResult, coin,
    coins,
};
use mixnet_contract::testable_mixnet_contract::MixnetContract;
use nym_contracts_common::Percent;
use nym_contracts_common::signing::{ContractMessageContent, MessageSignature};
use nym_contracts_common_testing::{
    AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt, ChainOpts,
    CommonStorageKeys, ContractFn, ContractOpts, ContractStorageWrapper, ContractTester,
    ContractTesterBuilder, DenomExt, PermissionedFn, QueryFn, RandExt, TEST_DENOM,
    TestableNymContract, addr,
};
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::nym_node::{NodeDetailsResponse, NodeOwnershipResponse, Role};
use nym_mixnet_contract_common::{
    CurrentIntervalResponse, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, DEFAULT_PROFIT_MARGIN_PERCENT,
    EpochId, Interval, NodeCostParams, NymNode, NymNodeBondingPayload, RoleAssignment,
    SignableNymNodeBondingMsg,
};
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NodeId, NodePerformanceSpecific, NodeResults,
    NymPerformanceContractError, QueryMsg,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::str::FromStr;

pub struct PerformanceContract;

impl TestableNymContract for PerformanceContract {
    const NAME: &'static str = "performance-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = NymPerformanceContractError;

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

    fn base_init_msg() -> Self::InitMsg {
        InstantiateMsg {
            mixnet_contract_address: addr("mixnet-contract").to_string(),
            authorised_network_monitors: vec![],
        }
    }

    fn init() -> ContractTester<Self>
    where
        Self: Sized,
    {
        let builder = ContractTesterBuilder::new().instantiate::<MixnetContract>(None);

        // we just instantiated it
        let mixnet_address = builder
            .well_known_contracts
            .get(MixnetContract::NAME)
            .unwrap()
            .clone();

        builder
            .instantiate::<Self>(Some(InstantiateMsg {
                mixnet_contract_address: mixnet_address.to_string(),
                authorised_network_monitors: vec![],
            }))
            .build()
    }
}

pub fn init_contract_tester() -> ContractTester<PerformanceContract> {
    PerformanceContract::init()
        .with_common_storage_key(CommonStorageKeys::Admin, storage_keys::CONTRACT_ADMIN)
}

// we need to be able to test instantiation, but for that we require
// deps in a state that already includes instantiated mixnet contract
pub(crate) struct PreInitContract {
    tester_builder: ContractTesterBuilder<PerformanceContract>,
    pub(crate) mixnet_contract_address: Addr,
    pub(crate) api: MockApi,
    storage: ContractStorageWrapper,
    placeholder_address: Addr,
}

#[allow(dead_code)]
impl PreInitContract {
    pub(crate) fn new() -> PreInitContract {
        let tester_builder =
            ContractTesterBuilder::<PerformanceContract>::new().instantiate::<MixnetContract>(None);

        let mixnet_contract = tester_builder
            .well_known_contracts
            .get(&MixnetContract::NAME)
            .unwrap();

        let api = tester_builder.api();
        let placeholder_address = api.addr_make("to-be-performance-contract");

        let storage = tester_builder.contract_storage_wrapper(&placeholder_address);

        PreInitContract {
            mixnet_contract_address: mixnet_contract.clone(),
            tester_builder,
            api,
            storage,
            placeholder_address,
        }
    }

    pub(crate) fn deps(&self) -> Deps<'_> {
        Deps {
            storage: &self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn deps_mut(&mut self) -> DepsMut<'_> {
        DepsMut {
            storage: &mut self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn querier(&self) -> QuerierWrapper<'_> {
        self.tester_builder.querier()
    }

    pub(crate) fn env(&self) -> Env {
        Env {
            contract: ContractInfo {
                address: self.placeholder_address.clone(),
            },
            ..mock_env()
        }
    }

    pub(crate) fn addr_make(&self, input: &str) -> Addr {
        self.api.addr_make(input)
    }

    pub(crate) fn write_to_mixnet_contract_storage(
        &mut self,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        self.set_contract_storage(address, key, value);
        Ok(())
    }

    pub(crate) fn write_to_mixnet_contract_storage_value<T: Serialize>(
        &mut self,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        let address = NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)?;

        self.set_contract_storage_value(address, key, value)
    }
}

impl ArbitraryContractStorageWriter for PreInitContract {
    fn set_contract_storage(
        &mut self,
        address: impl Into<String>,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) {
        self.storage
            .as_inner_storage_mut()
            .set_contract_storage(address, key, value);
    }
}

#[allow(dead_code)]
pub(crate) trait PerformanceContractTesterExt:
    ContractOpts<
        ExecuteMsg = ExecuteMsg,
        QueryMsg = QueryMsg,
        ContractError = NymPerformanceContractError,
    > + ChainOpts
    + AdminExt
    + DenomExt
    + RandExt
    + BankExt
    + ArbitraryContractStorageReader
    + ArbitraryContractStorageWriter
{
    fn mixnet_contract_address(&self) -> StdResult<Addr> {
        NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_contract_address
            .load(self.deps().storage)
    }

    fn execute_mixnet_contract(
        &mut self,
        sender: MessageInfo,
        msg: &nym_mixnet_contract_common::ExecuteMsg,
    ) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        self.execute_arbitrary_contract(address, sender, msg)
            .map_err(|err| {
                StdError::generic_err(format!("mixnet contract execution failure: {err}"))
            })?;
        Ok(())
    }

    fn read_from_mixnet_contract_storage<T: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> StdResult<T> {
        let address = self.mixnet_contract_address()?;

        self.must_read_value_from_contract_storage(address, key)
    }

    fn write_to_mixnet_contract_storage(
        &mut self,
        key: impl AsRef<[u8]>,
        value: impl AsRef<[u8]>,
    ) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        <Self as ArbitraryContractStorageWriter>::set_contract_storage(self, address, key, value);
        Ok(())
    }

    fn write_to_mixnet_contract_storage_value<T: Serialize>(
        &mut self,
        key: impl AsRef<[u8]>,
        value: &T,
    ) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        self.set_contract_storage_value(address, key, value)
    }

    fn current_mixnet_epoch(&self) -> StdResult<EpochId> {
        let address = self.mixnet_contract_address()?;

        Ok(self
            .deps()
            .querier
            .query_current_mixnet_interval(address.clone())?
            .current_epoch_absolute_id())
    }

    fn advance_mixnet_epoch(&mut self) -> StdResult<()> {
        let interval_details: CurrentIntervalResponse = self.query_arbitrary_contract(
            self.mixnet_contract_address()?,
            &nym_mixnet_contract_common::QueryMsg::GetCurrentIntervalDetails {},
        )?;
        let until_end = interval_details.time_until_current_epoch_end().as_secs();
        let timestamp = self.env().block.time.plus_seconds(until_end + 1);
        self.set_block_time(timestamp);
        self.next_block();

        // this was hardcoded in mixnet init
        let mixnet_rewarder = self.addr_make("rewarder");
        let rewarder = message_info(&mixnet_rewarder, &[]);
        self.execute_mixnet_contract(
            rewarder.clone(),
            &nym_mixnet_contract_common::ExecuteMsg::BeginEpochTransition {},
        )?;
        self.execute_mixnet_contract(
            rewarder.clone(),
            &nym_mixnet_contract_common::ExecuteMsg::ReconcileEpochEvents { limit: None },
        )?;

        for role in [
            Role::ExitGateway,
            Role::EntryGateway,
            Role::Layer1,
            Role::Layer2,
            Role::Layer3,
            Role::Standby,
        ] {
            self.execute_mixnet_contract(
                rewarder.clone(),
                &nym_mixnet_contract_common::ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role,
                        nodes: vec![],
                    },
                },
            )?;
        }
        Ok(())
    }

    fn set_mixnet_epoch(&mut self, epoch_id: EpochId) -> StdResult<()> {
        let address = self.mixnet_contract_address()?;

        let interval = self
            .deps()
            .querier
            .query_current_mixnet_interval(address.clone())?;

        let mut to_update = if interval.current_epoch_absolute_id() <= epoch_id {
            interval
        } else {
            Interval::init_interval(
                interval.epochs_in_interval(),
                interval.epoch_length(),
                &mock_env(),
            )
        };

        let current = to_update.current_epoch_absolute_id();
        let diff = epoch_id - current;
        for _ in 0..diff {
            to_update = to_update.advance_epoch();
        }
        self.set_contract_storage_value(&address, b"ci", &to_update)
    }

    fn authorise_network_monitor(
        &mut self,
        addr: &Addr,
    ) -> Result<(), NymPerformanceContractError> {
        let admin = self.admin_unchecked();
        self.execute_raw(
            admin,
            ExecuteMsg::AuthoriseNetworkMonitor {
                address: addr.to_string(),
            },
        )?;
        Ok(())
    }

    fn dummy_measurement_kind(&mut self) -> MeasurementKind {
        String::from("dummy")
    }

    fn define_dummy_measurement_kind(
        &mut self,
    ) -> Result<MeasurementKind, NymPerformanceContractError> {
        let admin = self.admin_unchecked();
        let measurement_kind = self.dummy_measurement_kind();

        self.execute_raw(
            admin,
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: measurement_kind.clone(),
            },
        )?;

        Ok(measurement_kind)
    }

    fn dummy_node_performance(&mut self) -> NodePerformanceSpecific {
        let node_id = self.bond_dummy_nymnode().unwrap();
        let measurement_kind = self.dummy_measurement_kind();
        NodePerformanceSpecific {
            node_id,
            performance: Percent::from_percentage_value(69).unwrap(),
            measurement_kind,
        }
    }

    fn retire_network_monitor(&mut self, addr: &Addr) -> Result<(), NymPerformanceContractError> {
        let admin = self.admin_unchecked();
        self.execute_raw(
            admin,
            ExecuteMsg::RetireNetworkMonitor {
                address: addr.to_string(),
            },
        )?;
        Ok(())
    }

    fn insert_epoch_performance(
        &mut self,
        addr: &Addr,
        epoch_id: EpochId,
        node_id: NodeId,
        measurement_kind: MeasurementKind,
        performance: Percent,
    ) -> Result<(), NymPerformanceContractError> {
        let env = self.env();
        NYM_PERFORMANCE_CONTRACT_STORAGE.submit_performance_data(
            self.deps_mut(),
            env,
            addr,
            epoch_id,
            NodePerformanceSpecific {
                node_id,
                performance,
                measurement_kind,
            },
        )
    }

    fn insert_performance(
        &mut self,
        addr: &Addr,
        node_id: NodeId,
        measurement_kind: MeasurementKind,
        performance: Percent,
    ) -> Result<(), NymPerformanceContractError> {
        let epoch_id = self.current_mixnet_epoch()?;

        self.insert_epoch_performance(addr, epoch_id, node_id, measurement_kind, performance)
    }

    // makes testing easier
    fn insert_raw_performance(
        &mut self,
        addr: &Addr,
        node_id: NodeId,
        measurement_kind: MeasurementKind,
        raw: &str,
    ) -> Result<(), NymPerformanceContractError> {
        self.insert_performance(
            addr,
            node_id,
            measurement_kind,
            Percent::from_str(raw).map_err(|err| {
                NymPerformanceContractError::StdErr(StdError::parse_err("Percent", err.to_string()))
            })?,
        )
    }

    fn read_raw_scores(
        &self,
        epoch_id: EpochId,
        node_id: NodeId,
    ) -> Result<NodeResults, NymPerformanceContractError> {
        let scores = NYM_PERFORMANCE_CONTRACT_STORAGE
            .performance_results
            .results
            .load(self.deps().storage, (epoch_id, node_id))?;
        Ok(scores)
    }

    fn bond_dummy_nymnode(&mut self) -> Result<NodeId, NymPerformanceContractError> {
        let node_owner = self.generate_account_with_balance();
        let pledge = coins(100_000000, TEST_DENOM);
        let keypair = ed25519::KeyPair::new(self.raw_rng());
        let identity_key = keypair.public_key().to_base58_string();

        let node = NymNode {
            host: "1.2.3.4".to_string(),
            custom_http_port: None,
            identity_key,
        };
        let cost_params = NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(DEFAULT_PROFIT_MARGIN_PERCENT)
                .unwrap(),
            interval_operating_cost: coin(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, TEST_DENOM),
        };
        // initial signing nonce is 0 for a new address
        let signing_nonce = 0;

        let payload = NymNodeBondingPayload::new(node.clone(), cost_params.clone());
        let content = ContractMessageContent::new(node_owner.clone(), pledge.clone(), payload);
        let msg = SignableNymNodeBondingMsg::new(signing_nonce, content);

        let owner_signature = keypair.private_key().sign(msg.to_plaintext()?);
        let owner_signature = MessageSignature::from(owner_signature.to_bytes().as_ref());

        self.execute_mixnet_contract(
            message_info(&node_owner, &pledge),
            &nym_mixnet_contract_common::ExecuteMsg::BondNymNode {
                node,
                cost_params,
                owner_signature,
            },
        )?;

        let bond: NodeOwnershipResponse = self.query_arbitrary_contract(
            self.mixnet_contract_address()?,
            &nym_mixnet_contract_common::QueryMsg::GetOwnedNymNode {
                address: node_owner.to_string(),
            },
        )?;

        Ok(bond.details.unwrap().bond_information.node_id)
    }

    fn unbond_nymnode(&mut self, node_id: NodeId) -> Result<(), NymPerformanceContractError> {
        let bond: NodeDetailsResponse = self.query_arbitrary_contract(
            self.mixnet_contract_address()?,
            &nym_mixnet_contract_common::QueryMsg::GetNymNodeDetails { node_id },
        )?;

        let node_owner = bond.details.unwrap().bond_information.owner;

        self.execute_mixnet_contract(
            message_info(&node_owner, &[]),
            &nym_mixnet_contract_common::ExecuteMsg::UnbondNymNode {},
        )?;

        self.advance_mixnet_epoch()?;
        Ok(())
    }
}

impl PerformanceContractTesterExt for ContractTester<PerformanceContract> {}
