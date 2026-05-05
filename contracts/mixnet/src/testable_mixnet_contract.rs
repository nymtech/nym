// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in test code
#![allow(clippy::unwrap_used)]

use crate::contract::{execute, instantiate, migrate, query};
use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{coin, coins, Addr, Decimal, MessageInfo, StdError, StdResult};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::nym_node::{NodeDetailsResponse, NodeOwnershipResponse, Role};
use mixnet_contract_common::reward_params::RewardedSetParams;
use mixnet_contract_common::{
    CurrentIntervalResponse, EpochId, ExecuteMsg, InitialRewardingParams, InstantiateMsg, Interval,
    MigrateMsg, MixnetContractQuerier, NodeCostParams, NodeId, NymNode, NymNodeBondingPayload,
    QueryMsg, RoleAssignment, SignableNymNodeBondingMsg, DEFAULT_INTERVAL_OPERATING_COST_AMOUNT,
    DEFAULT_PROFIT_MARGIN_PERCENT,
};
use nym_contracts_common::signing::{ContractMessageContent, MessageSignature};
use nym_contracts_common::Percent;
use nym_contracts_common_testing::{
    mock_dependencies, ArbitraryContractStorageReader, ArbitraryContractStorageWriter, BankExt,
    ChainOpts, ContractFn, ContractTester, PermissionedFn, QueryFn, RandExt, TEST_DENOM,
};
use nym_crypto::asymmetric::ed25519;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

pub use nym_contracts_common_testing::TestableNymContract;

pub struct MixnetContract;

fn initial_rewarded_set_params() -> RewardedSetParams {
    RewardedSetParams {
        entry_gateways: 50,
        exit_gateways: 70,
        mixnodes: 120,
        standby: 50,
    }
}

fn initial_rewarding_params() -> InitialRewardingParams {
    let reward_pool = 250_000_000_000_000u128;
    let staking_supply = 100_000_000_000_000u128;

    InitialRewardingParams {
        initial_reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(), // 250M * 1M (we're expressing it all in base tokens)
        initial_staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(), // 100M * 1M
        staking_supply_scale_factor: Percent::hundred(),
        sybil_resistance: Percent::from_percentage_value(30).unwrap(),
        active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
        interval_pool_emission: Percent::from_percentage_value(2).unwrap(),
        rewarded_set_params: initial_rewarded_set_params(),
    }
}

impl TestableNymContract for MixnetContract {
    const NAME: &'static str = "mixnet-contract";
    type InitMsg = InstantiateMsg;
    type ExecuteMsg = ExecuteMsg;
    type QueryMsg = QueryMsg;
    type MigrateMsg = MigrateMsg;
    type ContractError = MixnetContractError;

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
        let deps = mock_dependencies();
        InstantiateMsg {
            rewarding_validator_address: deps.api.addr_make("rewarder").to_string(),
            vesting_contract_address: deps.api.addr_make("vesting-contract").to_string(),
            rewarding_denom: TEST_DENOM.to_string(),
            epochs_in_interval: 720,
            epoch_duration: Duration::from_secs(60 * 60),
            initial_rewarding_params: initial_rewarding_params(),
            current_nym_node_version: "1.1.10".to_string(),
            version_score_weights: Default::default(),
            version_score_params: Default::default(),
            profit_margin: Default::default(),
            interval_operating_cost: Default::default(),
            key_validity_in_epochs: None,
        }
    }
}

pub trait EmbeddedMixnetContractExt:
    ChainOpts + ArbitraryContractStorageWriter + ArbitraryContractStorageReader + RandExt + BankExt
{
    fn mixnet_contract_address(&self) -> StdResult<Addr>;

    fn execute_mixnet_contract(&mut self, sender: MessageInfo, msg: &ExecuteMsg) -> StdResult<()> {
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
            &QueryMsg::GetCurrentIntervalDetails {},
        )?;
        let until_end = interval_details.time_until_current_epoch_end().as_secs();
        let timestamp = self.env().block.time.plus_seconds(until_end + 1);
        self.set_block_time(timestamp);
        self.next_block();

        // this was hardcoded in mixnet init
        let mixnet_rewarder = self.addr_make("rewarder");
        let rewarder = message_info(&mixnet_rewarder, &[]);
        self.execute_mixnet_contract(rewarder.clone(), &ExecuteMsg::BeginEpochTransition {})?;
        self.execute_mixnet_contract(
            rewarder.clone(),
            &ExecuteMsg::ReconcileEpochEvents { limit: None },
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
                &ExecuteMsg::AssignRoles {
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

    fn bond_dummy_nymnode(&mut self) -> Result<NodeId, StdError> {
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
            &ExecuteMsg::BondNymNode {
                node,
                cost_params,
                owner_signature,
            },
        )?;

        let bond: NodeOwnershipResponse = self.query_arbitrary_contract(
            self.mixnet_contract_address()?,
            &QueryMsg::GetOwnedNymNode {
                address: node_owner.to_string(),
            },
        )?;

        Ok(bond.details.unwrap().bond_information.node_id)
    }

    fn unbond_nymnode(&mut self, node_id: NodeId) -> Result<(), StdError> {
        let bond: NodeDetailsResponse = self.query_arbitrary_contract(
            self.mixnet_contract_address()?,
            &QueryMsg::GetNymNodeDetails { node_id },
        )?;

        let node_owner = bond.details.unwrap().bond_information.owner;

        self.execute_mixnet_contract(
            message_info(&node_owner, &[]),
            &ExecuteMsg::UnbondNymNode {},
        )?;

        self.advance_mixnet_epoch()?;
        Ok(())
    }
}

impl<C> EmbeddedMixnetContractExt for ContractTester<C>
where
    C: TestableNymContract,
{
    fn mixnet_contract_address(&self) -> StdResult<Addr> {
        self.well_known_contracts
            .get(MixnetContract::NAME)
            .ok_or_else(|| StdError::generic_err("mixnet contract not part of the tester"))
            .cloned()
    }
}
