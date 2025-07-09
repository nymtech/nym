// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::{execute, instantiate, migrate, query};
use crate::helpers::MixnetContractQuerier;
use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use cosmwasm_std::testing::{message_info, mock_env, MockApi};
use cosmwasm_std::{
    coin, coins, Addr, Binary, ContractInfo, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    StdError, StdResult,
};
use cw_storage_plus::PrimaryKey;
use mixnet_contract::testable_mixnet_contract::MixnetContract;
use nym_contracts_common::signing::{ContractMessageContent, MessageSignature};
use nym_contracts_common::Percent;
use nym_contracts_common_testing::{
    addr, AdminExt, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt,
    ChainOpts, CommonStorageKeys, ContractFn, ContractOpts, ContractStorageWrapper, ContractTester,
    ContractTesterBuilder, DenomExt, PermissionedFn, QueryFn, RandExt, TestableNymContract,
    TEST_DENOM,
};
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::nym_node::{NodeDetailsResponse, NodeOwnershipResponse, Role};
use nym_mixnet_contract_common::{
    CurrentIntervalResponse, EpochId, Interval, MixNode, MixNodeBond, MixnodeDetailsResponse,
    NodeCostParams, NodeRewarding, NymNode, NymNodeBondingPayload, RoleAssignment,
    SignableNymNodeBondingMsg, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT,
    DEFAULT_PROFIT_MARGIN_PERCENT,
};
use nym_performance_contract_common::constants::storage_keys;
use nym_performance_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NodeId, NodePerformance, NodeResults,
    NymPerformanceContractError, QueryMsg,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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

    pub(crate) fn deps(&self) -> Deps {
        Deps {
            storage: &self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn deps_mut(&mut self) -> DepsMut {
        DepsMut {
            storage: &mut self.storage,
            api: &self.api,
            querier: self.tester_builder.querier(),
        }
    }

    pub(crate) fn querier(&self) -> QuerierWrapper {
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

    fn dummy_node_performance(&mut self) -> NodePerformance {
        let node_id = self.bond_dummy_nymnode().unwrap();
        NodePerformance {
            node_id,
            performance: Percent::from_percentage_value(69).unwrap(),
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
        performance: Percent,
    ) -> Result<(), NymPerformanceContractError> {
        let env = self.env();
        NYM_PERFORMANCE_CONTRACT_STORAGE.submit_performance_data(
            self.deps_mut(),
            env,
            addr,
            epoch_id,
            NodePerformance {
                node_id,
                performance,
            },
        )
    }

    fn insert_performance(
        &mut self,
        addr: &Addr,
        node_id: NodeId,
        performance: Percent,
    ) -> Result<(), NymPerformanceContractError> {
        let epoch_id = self.current_mixnet_epoch()?;

        self.insert_epoch_performance(addr, epoch_id, node_id, performance)
    }

    // makes testing easier
    fn insert_raw_performance(
        &mut self,
        addr: &Addr,
        node_id: NodeId,
        raw: &str,
    ) -> Result<(), NymPerformanceContractError> {
        self.insert_performance(
            addr,
            node_id,
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

    fn bond_dummy_legacy_mixnode(&mut self) -> Result<NodeId, NymPerformanceContractError> {
        #[derive(Deserialize, Serialize)]
        pub(crate) struct UniqueRef<T> {
            // note, we collapse the pk - combining everything under the namespace - even if it is composite
            pk: Binary,
            value: T,
        }

        // there's no proper Execute flow for this anymore, so we have to "hack" the storage a bit,
        // ensuring all invariants still hold
        let owner = self.generate_account_with_balance();

        let mixnode = MixNode {
            host: "1.2.3.4".to_string(),
            mix_port: 123,
            verloc_port: 123,
            http_api_port: 123,
            sphinx_key: "aaaa".to_string(),
            identity_key: "bbbbb".to_string(),
            version: "ccc".to_string(),
        };
        let cost_params = NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(DEFAULT_PROFIT_MARGIN_PERCENT)
                .unwrap(),
            interval_operating_cost: coin(DEFAULT_INTERVAL_OPERATING_COST_AMOUNT, TEST_DENOM),
        };

        // adjust node counter
        let node_id_counter: u32 = self.read_from_mixnet_contract_storage("nic")?;
        let node_id = node_id_counter + 1;
        self.write_to_mixnet_contract_storage_value("nic", &node_id)?;

        let current_epoch = self.current_mixnet_epoch()?;
        let pledge = coin(100_000000, TEST_DENOM);
        let mixnode_rewarding =
            NodeRewarding::initialise_new(cost_params, &pledge, current_epoch).unwrap();
        let env = self.env();
        let mixnode_bond = MixNodeBond {
            mix_id: node_id,
            owner,
            original_pledge: pledge,
            mix_node: mixnode,
            proxy: None,
            bonding_height: env.block.height,
            is_unbonding: false,
        };

        // save to the main mixnode storage
        self.set_contract_map_value(
            self.mixnet_contract_address()?,
            "mnn",
            node_id,
            &mixnode_bond,
        )?;
        // update indices
        let pk = node_id.joined_key();
        let unique_ref = UniqueRef {
            pk: pk.into(),
            value: mixnode_bond.clone(),
        };

        // owner index
        let idx = mixnode_bond.owner.clone();
        self.set_contract_map_value(self.mixnet_contract_address()?, "mno", idx, &unique_ref)?;

        // identity key index
        let idx = mixnode_bond.mix_node.identity_key.clone();
        self.set_contract_map_value(self.mixnet_contract_address()?, "mni", idx, &unique_ref)?;

        // sphinx key index
        let idx = mixnode_bond.mix_node.sphinx_key.clone();
        self.set_contract_map_value(self.mixnet_contract_address()?, "mns", idx, &unique_ref)?;

        // update rewarding data
        self.set_contract_map_value(
            self.mixnet_contract_address()?,
            "mnr",
            node_id,
            &mixnode_rewarding,
        )?;

        Ok(node_id)
    }

    fn unbond_legacy_mixnode(
        &mut self,
        node_id: NodeId,
    ) -> Result<(), NymPerformanceContractError> {
        let bond: MixnodeDetailsResponse = self.query_arbitrary_contract(
            self.mixnet_contract_address()?,
            &nym_mixnet_contract_common::QueryMsg::GetMixnodeDetails { mix_id: node_id },
        )?;

        let node_owner = bond.mixnode_details.unwrap().bond_information.owner;

        self.execute_mixnet_contract(
            message_info(&node_owner, &[]),
            &nym_mixnet_contract_common::ExecuteMsg::UnbondMixnode {},
        )?;

        self.advance_mixnet_epoch()?;
        Ok(())
    }
}

impl PerformanceContractTesterExt for ContractTester<PerformanceContract> {}
