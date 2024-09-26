// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
pub mod fixtures;
pub(crate) mod legacy;

#[cfg(test)]
pub mod test_helpers {
    use crate::constants;
    use crate::contract::{execute, instantiate};
    use crate::delegations::queries::query_node_delegations_paged;
    use crate::delegations::storage as delegations_storage;
    use crate::delegations::transactions::try_delegate_to_node;
    use crate::interval::transactions::{
        perform_pending_epoch_actions, perform_pending_interval_actions, try_begin_epoch_transition,
    };
    use crate::interval::{pending_events, storage as interval_storage};
    use crate::mixnet_contract_settings::queries::query_contract_settings_params;
    use crate::mixnet_contract_settings::storage::{
        self as mixnet_params_storage, minimum_node_pledge,
    };
    use crate::mixnet_contract_settings::storage::{rewarding_denom, rewarding_validator_address};
    use crate::mixnodes::helpers::get_mixnode_details_by_id;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::mixnodes::storage::mixnode_bonds;
    use crate::mixnodes::transactions::try_remove_mixnode;
    use crate::nodes::helpers::{
        get_node_details_by_id, get_node_details_by_identity, must_get_node_bond_by_owner,
    };
    use crate::nodes::storage as nymnodes_storage;
    use crate::nodes::storage::helpers::RoleStorageBucket;
    use crate::nodes::storage::rewarded_set::{ACTIVE_ROLES_BUCKET, ROLES, ROLES_METADATA};
    use crate::nodes::storage::{read_assigned_roles, save_assignment, swap_active_role_bucket};
    use crate::nodes::transactions::{try_add_nym_node, try_remove_nym_node};
    use crate::rewards::helpers::expensive_role_lookup;
    use crate::rewards::queries::{
        query_pending_delegator_reward, query_pending_mixnode_operator_reward,
    };
    use crate::rewards::storage as rewards_storage;
    use crate::rewards::storage::RewardingStorage;
    use crate::rewards::transactions::try_reward_node;
    use crate::signing::storage as signing_storage;
    use crate::support::helpers::ensure_no_existing_bond;
    use crate::support::tests;
    use crate::support::tests::fixtures::{
        good_gateway_pledge, good_mixnode_pledge, good_node_plegge, TEST_COIN_DENOM,
    };
    use crate::support::tests::{legacy, test_helpers};
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::{coin, coins, Addr, BankMsg, CosmosMsg, Storage};
    use cosmwasm_std::{Coin, Order};
    use cosmwasm_std::{Decimal, Empty, MemoryStorage};
    use cosmwasm_std::{Deps, OwnedDeps};
    use cosmwasm_std::{DepsMut, MessageInfo};
    use cosmwasm_std::{Env, Response, Timestamp, Uint128};
    use mixnet_contract_common::error::MixnetContractError;
    use mixnet_contract_common::events::{
        may_find_attribute, MixnetEventType, DELEGATES_REWARD_KEY, OPERATOR_REWARD_KEY,
    };
    use mixnet_contract_common::mixnode::{NodeRewarding, UnbondedMixnode};
    use mixnet_contract_common::nym_node::{RewardedSetMetadata, Role};
    use mixnet_contract_common::pending_events::{PendingEpochEventData, PendingIntervalEventData};
    use mixnet_contract_common::reward_params::{
        NodeRewardingParameters, Performance, RewardedSetParams, RewardingParams, WorkFactor,
    };
    use mixnet_contract_common::rewarding::simulator::simulated_node::SimulatedNode;
    use mixnet_contract_common::rewarding::simulator::Simulator;
    use mixnet_contract_common::rewarding::RewardDistribution;
    use mixnet_contract_common::{
        ContractStateParams, Delegation, EpochEventId, EpochState, EpochStatus, ExecuteMsg,
        Gateway, GatewayBondingPayload, IdentityKey, InitialRewardingParams, InstantiateMsg,
        Interval, MixNode, MixNodeBond, MixNodeDetails, MixnodeBondingPayload, NodeId, NymNode,
        NymNodeBond, NymNodeBondingPayload, NymNodeDetails, OperatingCostRange, Percent,
        ProfitMarginRange, RoleAssignment, SignableGatewayBondingMsg, SignableMixNodeBondingMsg,
        SignableNymNodeBondingMsg,
    };
    use nym_contracts_common::signing::{
        ContractMessageContent, MessageSignature, SignableMessage, SigningAlgorithm, SigningPurpose,
    };
    use nym_crypto::asymmetric::identity;
    use nym_crypto::asymmetric::identity::KeyPair;
    use rand_chacha::rand_core::{CryptoRng, RngCore, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use serde::Serialize;
    use std::collections::HashMap;
    use std::fmt::Debug;
    use std::str::FromStr;
    use std::time::Duration;

    pub(crate) trait ExtractBankMsg {
        fn unwrap_bank_msg(self) -> Option<BankMsg>;
    }

    impl ExtractBankMsg for Response {
        fn unwrap_bank_msg(self) -> Option<BankMsg> {
            for msg in self.messages {
                match msg.msg {
                    CosmosMsg::Bank(bank_msg) => return Some(bank_msg),
                    _ => continue,
                }
            }

            None
        }
    }

    #[allow(clippy::enum_variant_names)]
    pub enum NodeQueryType {
        ById(NodeId),
        ByIdentity(IdentityKey),
        ByOwner(Addr),
    }

    impl From<NodeId> for NodeQueryType {
        fn from(value: NodeId) -> Self {
            NodeQueryType::ById(value)
        }
    }

    impl From<IdentityKey> for NodeQueryType {
        fn from(value: IdentityKey) -> Self {
            NodeQueryType::ByIdentity(value)
        }
    }
    impl From<Addr> for NodeQueryType {
        fn from(value: Addr) -> Self {
            NodeQueryType::ByOwner(value)
        }
    }

    pub fn assert_eq_with_leeway(a: Uint128, b: Uint128, leeway: Uint128) {
        if a > b {
            assert!(a - b <= leeway)
        } else {
            assert!(b - a <= leeway)
        }
    }

    pub fn assert_decimals(a: Decimal, b: Decimal) {
        let epsilon = Decimal::from_ratio(1u128, 100_000_000u128);
        if a > b {
            assert!(a - b < epsilon, "{} != {}", a, b)
        } else {
            assert!(b - a < epsilon, "{} != {}", a, b)
        }
    }

    pub struct TestSetup {
        pub deps: OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>>,
        pub env: Env,
        pub rng: ChaCha20Rng,

        pub rewarding_validator: MessageInfo,
        pub owner: MessageInfo,
    }

    impl TestSetup {
        pub fn new() -> Self {
            let deps = init_contract();
            let rewarding_validator_address =
                rewarding_validator_address(deps.as_ref().storage).unwrap();
            let owner = mixnet_params_storage::ADMIN
                .query_admin(deps.as_ref())
                .unwrap()
                .admin
                .unwrap();

            TestSetup {
                deps,
                env: mock_env(),
                rng: test_rng(),
                rewarding_validator: mock_info(rewarding_validator_address.as_ref(), &[]),
                owner: mock_info(owner.as_str(), &[]),
            }
        }

        pub fn deps(&self) -> Deps<'_> {
            self.deps.as_ref()
        }

        pub fn deps_mut(&mut self) -> DepsMut<'_> {
            self.deps.as_mut()
        }

        pub fn env(&self) -> Env {
            self.env.clone()
        }

        pub fn execute(
            &mut self,
            info: MessageInfo,
            msg: ExecuteMsg,
        ) -> Result<Response, MixnetContractError> {
            let env = self.env.clone();
            execute(self.deps_mut(), env, info, msg)
        }

        #[allow(unused)]
        pub fn execute_no_funds(
            &mut self,
            sender: impl Into<String>,
            msg: ExecuteMsg,
        ) -> Result<Response, MixnetContractError> {
            self.execute(self.mock_info(sender), msg)
        }

        pub fn execute_fn<F>(
            &mut self,
            exec_fn: F,
            info: MessageInfo,
        ) -> Result<Response, MixnetContractError>
        where
            F: FnOnce(DepsMut<'_>, Env, MessageInfo) -> Result<Response, MixnetContractError>,
        {
            let env = self.env().clone();
            exec_fn(self.deps_mut(), env, info)
        }

        #[allow(unused)]
        pub fn execute_fn_no_funds<F>(
            &mut self,
            exec_fn: F,
            sender: impl Into<String>,
        ) -> Result<Response, MixnetContractError>
        where
            F: FnOnce(DepsMut<'_>, Env, MessageInfo) -> Result<Response, MixnetContractError>,
        {
            let info = self.mock_info(sender);
            self.execute_fn(exec_fn, info)
        }

        #[track_caller]
        pub fn assert_simple_execution<F>(&mut self, exec_fn: F, info: MessageInfo) -> Response
        where
            F: FnOnce(DepsMut<'_>, Env, MessageInfo) -> Result<Response, MixnetContractError>,
        {
            let caller = std::panic::Location::caller();
            self.execute_fn(exec_fn, info)
                .unwrap_or_else(|err| panic!("{caller} failed with: '{err}' ({err:?})"))
        }

        #[allow(unused)]
        #[track_caller]
        pub fn assert_simple_execution_no_funds<F>(
            &mut self,
            exec_fn: F,
            sender: impl Into<String>,
        ) -> Response
        where
            F: FnOnce(DepsMut<'_>, Env, MessageInfo) -> Result<Response, MixnetContractError>,
        {
            let caller = std::panic::Location::caller();
            self.execute_fn_no_funds(exec_fn, sender)
                .unwrap_or_else(|err| panic!("{caller} failed with: '{err}' ({err:?})"))
        }

        pub fn update_profit_margin_range(&mut self, range: ProfitMarginRange) {
            let current = query_contract_settings_params(self.deps()).unwrap();

            self.execute(
                self.owner(),
                ExecuteMsg::UpdateContractStateParams {
                    updated_parameters: ContractStateParams {
                        profit_margin: range,
                        ..current
                    },
                },
            )
            .unwrap();
        }

        pub fn update_operating_cost_range(&mut self, range: OperatingCostRange) {
            let current = query_contract_settings_params(self.deps()).unwrap();

            self.execute(
                self.owner(),
                ExecuteMsg::UpdateContractStateParams {
                    updated_parameters: ContractStateParams {
                        interval_operating_cost: range,
                        ..current
                    },
                },
            )
            .unwrap();
        }

        pub fn get_node_id(&self, query_type: impl Into<NodeQueryType>) -> NodeId {
            match query_type.into() {
                NodeQueryType::ById(id) => id,
                NodeQueryType::ByIdentity(identity) => {
                    get_node_details_by_identity(&self.deps.storage, identity)
                        .unwrap()
                        .unwrap()
                        .node_id()
                }
                NodeQueryType::ByOwner(owner) => {
                    must_get_node_bond_by_owner(&self.deps.storage, &owner)
                        .unwrap()
                        .node_id
                }
            }
        }

        #[allow(unused)]
        pub fn mock_info(&self, sender: impl Into<String>) -> MessageInfo {
            mock_info(&sender.into(), &[])
        }

        pub fn rewarding_validator(&self) -> MessageInfo {
            self.rewarding_validator.clone()
        }

        pub fn rewarding_params(&self) -> RewardingParams {
            rewards_storage::REWARDING_PARAMS
                .load(self.deps().storage)
                .unwrap()
        }

        pub fn owner(&self) -> MessageInfo {
            self.owner.clone()
        }

        pub fn coin(&self, amount: u128) -> Coin {
            coin(amount, rewarding_denom(self.deps().storage).unwrap())
        }

        pub fn coins(&self, amount: u128) -> Vec<Coin> {
            coins(amount, rewarding_denom(self.deps().storage).unwrap())
        }

        pub fn current_interval(&self) -> Interval {
            interval_storage::current_interval(self.deps().storage).unwrap()
        }

        pub fn active_roles_bucket(&self) -> RoleStorageBucket {
            ACTIVE_ROLES_BUCKET.load(self.deps().storage).unwrap()
        }

        #[allow(unused)]
        pub fn active_roles_metadata(&self) -> RewardedSetMetadata {
            let bucket = self.active_roles_bucket().other();
            ROLES_METADATA
                .load(self.deps().storage, bucket as u8)
                .unwrap()
        }

        pub fn inactive_roles_metadata(&self) -> RewardedSetMetadata {
            let bucket = self.active_roles_bucket().other();
            ROLES_METADATA
                .load(self.deps().storage, bucket as u8)
                .unwrap()
        }

        #[allow(unused)]
        pub fn active_roles(&self, role: Role) -> Vec<NodeId> {
            let bucket = self.active_roles_bucket().other();
            ROLES
                .load(self.deps().storage, (bucket as u8, role))
                .unwrap()
        }

        pub fn inactive_roles(&self, role: Role) -> Vec<NodeId> {
            let bucket = self.active_roles_bucket().other();
            ROLES
                .load(self.deps().storage, (bucket as u8, role))
                .unwrap()
        }

        pub fn max_role_count(&self, role: Role) -> u32 {
            RewardingStorage::load()
                .global_rewarding_params
                .load(self.deps().storage)
                .unwrap()
                .rewarded_set
                .maximum_role_count(role)
        }

        pub fn set_pending_pledge_change(
            &mut self,
            mix_id: NodeId,
            event_id: Option<EpochEventId>,
        ) {
            let mut changes = mixnodes_storage::PENDING_MIXNODE_CHANGES
                .load(self.deps().storage, mix_id)
                .unwrap_or_default();
            changes.pledge_change = Some(event_id.unwrap_or(12345));

            mixnodes_storage::PENDING_MIXNODE_CHANGES
                .save(self.deps_mut().storage, mix_id, &changes)
                .unwrap();
        }

        pub fn immediately_assign_lowest_mix_layer(&mut self, node_id: NodeId) -> Role {
            let active_bucket = ACTIVE_ROLES_BUCKET.load(&self.deps.storage).unwrap();

            let mut layer1 = read_assigned_roles(&self.deps.storage, Role::Layer1).unwrap();
            let mut layer2 = read_assigned_roles(&self.deps.storage, Role::Layer2).unwrap();
            let mut layer3 = read_assigned_roles(&self.deps.storage, Role::Layer3).unwrap();
            let l1 = layer1.len();
            let l2 = layer2.len();
            let l3 = layer3.len();

            if l1 <= l2 && l1 <= l3 {
                layer1.push(node_id);
                ROLES
                    .save(
                        &mut self.deps.storage,
                        (active_bucket as u8, Role::Layer1),
                        &layer1,
                    )
                    .unwrap();

                Role::Layer1
            } else if l2 <= l3 && l2 <= l1 {
                layer2.push(node_id);
                ROLES
                    .save(
                        &mut self.deps.storage,
                        (active_bucket as u8, Role::Layer2),
                        &layer2,
                    )
                    .unwrap();

                Role::Layer2
            } else {
                layer3.push(node_id);
                ROLES
                    .save(
                        &mut self.deps.storage,
                        (active_bucket as u8, Role::Layer3),
                        &layer3,
                    )
                    .unwrap();

                Role::Layer3
            }
        }

        pub fn add_rewarded_set_nymnode(
            &mut self,
            owner: &str,
            stake: Option<Uint128>,
        ) -> (NymNodeBond, MessageSignature, KeyPair) {
            let res = self.add_nymnode(owner, stake);
            let id = res.0.node_id;
            self.immediately_assign_lowest_mix_layer(id);

            res
        }

        pub fn add_rewarded_set_nymnode_id(
            &mut self,
            owner: &str,
            stake: Option<Uint128>,
        ) -> NodeId {
            self.add_rewarded_set_nymnode(owner, stake).0.node_id
        }

        pub fn add_nymnode(
            &mut self,
            owner: &str,
            stake: Option<Uint128>,
        ) -> (NymNodeBond, MessageSignature, KeyPair) {
            let stake = self.make_node_pledge(stake);
            let (node, owner_signature, keypair) =
                self.node_with_signature(owner, Some(stake.clone()));

            let info = mock_info(owner, stake.as_ref());
            let env = self.env();

            try_add_nym_node(
                self.deps_mut(),
                env,
                info.clone(),
                node,
                tests::fixtures::node_cost_params_fixture(),
                owner_signature.clone(),
            )
            .unwrap();

            let bond = must_get_node_bond_by_owner(&self.deps.storage, &info.sender).unwrap();

            (bond, owner_signature, keypair)
        }

        pub fn add_dummy_nymnode(&mut self, owner: &str, stake: Option<Uint128>) -> NodeId {
            self.add_nymnode(owner, stake).0.node_id
        }

        #[allow(unused)]
        pub fn add_dummy_nym_node_with_keypair(
            &mut self,
            owner: &str,
            stake: Option<Uint128>,
        ) -> (NodeId, identity::KeyPair) {
            let (bond, _, keypair) = self.add_nymnode(owner, stake);
            (bond.node_id, keypair)
        }

        #[track_caller]
        pub fn add_legacy_mixnode(&mut self, owner: &str, stake: Option<Uint128>) -> NodeId {
            let stake = self.make_mix_pledge(stake);
            let (mixnode, _, _) = self.mixnode_with_signature(owner, Some(stake.clone()));

            let info = mock_info(owner, stake.as_ref());
            let env = self.env();

            ensure_no_existing_bond(&info.sender, &self.deps.storage).unwrap();
            signing_storage::increment_signing_nonce(&mut self.deps.storage, info.sender.clone())
                .unwrap();
            let node_id = legacy::save_new_mixnode(
                &mut self.deps.storage,
                env,
                mixnode,
                tests::fixtures::node_cost_params_fixture(),
                info.sender,
                info.funds[0].clone(),
            )
            .unwrap();

            node_id
        }

        pub fn add_rewarded_legacy_mixnode(
            &mut self,
            owner: &str,
            stake: Option<Uint128>,
        ) -> NodeId {
            let node_id = self.add_legacy_mixnode(owner, stake);
            self.immediately_assign_lowest_mix_layer(node_id);

            node_id
        }

        pub fn add_legacy_gateway(&mut self, sender: &str, stake: Option<Uint128>) -> IdentityKey {
            let stake = self.make_gateway_pledge(stake);
            let (gateway, _) = self.gateway_with_signature(sender, Some(stake.clone()));

            let env = self.env();
            let info = mock_info(sender, &stake);

            legacy::save_new_gateway(
                &mut self.deps.storage,
                env,
                gateway,
                info.sender,
                info.funds[0].clone(),
            )
            .unwrap()
        }

        pub fn save_legacy_gateway(&mut self, gateway: Gateway, info: &MessageInfo) {
            let env = self.env();

            legacy::save_new_gateway(
                &mut self.deps.storage,
                env,
                gateway,
                info.sender.clone(),
                info.funds[0].clone(),
            )
            .unwrap();
        }

        pub fn add_dummy_mixnodes(&mut self, n: usize) {
            for i in 0..n {
                self.add_legacy_mixnode(&format!("owner{i}"), None);
            }
        }

        pub fn add_dummy_gateways(&mut self, n: usize) {
            for i in 0..n {
                self.add_legacy_gateway(&format!("owner{i}"), None);
            }
        }

        pub fn make_node_pledge(&self, stake: Option<Uint128>) -> Vec<Coin> {
            let stake = match stake {
                Some(amount) => {
                    let denom = rewarding_denom(self.deps().storage).unwrap();
                    Coin { denom, amount }
                }
                None => minimum_node_pledge(self.deps.as_ref().storage).unwrap(),
            };
            vec![stake]
        }

        pub fn make_mix_pledge(&self, stake: Option<Uint128>) -> Vec<Coin> {
            self.make_node_pledge(stake)
        }

        pub fn make_gateway_pledge(&self, stake: Option<Uint128>) -> Vec<Coin> {
            self.make_node_pledge(stake)
        }

        pub fn mixnode_by_id(&self, node_id: NodeId) -> Option<MixNodeDetails> {
            get_mixnode_details_by_id(self.deps().storage, node_id).unwrap()
        }

        pub fn nymnode_by_id(&self, node_id: NodeId) -> Option<NymNodeDetails> {
            get_node_details_by_id(self.deps().storage, node_id).unwrap()
        }

        pub fn mixnode_bonding_signature(
            &mut self,
            key: &identity::PrivateKey,
            owner: &str,
            mixnode: MixNode,
            stake: Option<Uint128>,
        ) -> MessageSignature {
            let stake = self.make_mix_pledge(stake);
            let msg = mixnode_bonding_sign_payload(self.deps(), owner, mixnode, stake);
            ed25519_sign_message(msg, key)
        }

        pub fn add_dummy_mixnode_with_keypair(
            &mut self,
            owner: &str,
            stake: Option<Uint128>,
        ) -> (NodeId, identity::KeyPair) {
            let stake = self.make_mix_pledge(stake);
            let (mixnode, _, keypair) = self.mixnode_with_signature(owner, Some(stake.clone()));

            let info = mock_info(owner, stake.as_ref());
            let env = self.env();

            ensure_no_existing_bond(&info.sender, &self.deps.storage).unwrap();
            signing_storage::increment_signing_nonce(&mut self.deps.storage, info.sender.clone())
                .unwrap();
            let node_id = legacy::save_new_mixnode(
                &mut self.deps.storage,
                env,
                mixnode,
                tests::fixtures::node_cost_params_fixture(),
                info.sender,
                info.funds[0].clone(),
            )
            .unwrap();

            (node_id, keypair)
        }

        pub fn node_with_signature(
            &mut self,
            sender: &str,
            stake: Option<Vec<Coin>>,
        ) -> (NymNode, MessageSignature, KeyPair) {
            let stake = stake.unwrap_or(good_node_plegge());

            let keypair = identity::KeyPair::new(&mut self.rng);
            let identity_key = keypair.public_key().to_base58_string();

            let node = NymNode {
                host: "1.2.3.4".to_string(),
                custom_http_port: None,
                identity_key,
            };
            let msg = nymnode_bonding_sign_payload(self.deps(), sender, node.clone(), stake);
            let owner_signature = ed25519_sign_message(msg, keypair.private_key());

            (node, owner_signature, keypair)
        }

        pub fn mixnode_with_signature(
            &mut self,
            sender: &str,
            stake: Option<Vec<Coin>>,
        ) -> (MixNode, MessageSignature, KeyPair) {
            let stake = stake.unwrap_or(good_mixnode_pledge());

            let keypair = identity::KeyPair::new(&mut self.rng);
            let identity_key = keypair.public_key().to_base58_string();
            let legit_sphinx_keys = nym_crypto::asymmetric::encryption::KeyPair::new(&mut self.rng);

            let mixnode = MixNode {
                identity_key,
                sphinx_key: legit_sphinx_keys.public_key().to_base58_string(),
                ..tests::fixtures::mix_node_fixture()
            };
            let msg = mixnode_bonding_sign_payload(self.deps(), sender, mixnode.clone(), stake);
            let owner_signature = ed25519_sign_message(msg, keypair.private_key());

            (mixnode, owner_signature, keypair)
        }

        pub fn gateway_with_signature(
            &mut self,
            sender: impl Into<String>,
            stake: Option<Vec<Coin>>,
        ) -> (Gateway, MessageSignature) {
            let stake = stake.unwrap_or(good_gateway_pledge());

            let keypair = identity::KeyPair::new(&mut self.rng);
            let identity_key = keypair.public_key().to_base58_string();
            let legit_sphinx_keys = nym_crypto::asymmetric::encryption::KeyPair::new(&mut self.rng);

            let gateway = Gateway {
                identity_key,
                sphinx_key: legit_sphinx_keys.public_key().to_base58_string(),
                ..tests::fixtures::gateway_fixture()
            };

            let msg = gateway_bonding_sign_payload(
                self.deps(),
                sender.into().as_str(),
                gateway.clone(),
                stake,
            );
            let owner_signature = ed25519_sign_message(msg, keypair.private_key());

            (gateway, owner_signature)
        }

        #[track_caller]
        pub fn start_unbonding_mixnode(&mut self, mix_id: NodeId) {
            let bond_details = mixnodes_storage::mixnode_bonds()
                .load(self.deps().storage, mix_id)
                .unwrap();

            let env = self.env();
            try_remove_mixnode(
                self.deps_mut(),
                env,
                mock_info(bond_details.owner.as_str(), &[]),
            )
            .unwrap();
        }

        #[track_caller]
        pub fn start_unbonding_nymnode(&mut self, node_id: NodeId) {
            let bond_details = nymnodes_storage::nym_nodes()
                .load(self.deps().storage, node_id)
                .unwrap();

            let env = self.env();
            try_remove_nym_node(
                self.deps_mut(),
                env,
                mock_info(bond_details.owner.as_str(), &[]),
            )
            .unwrap();
        }

        #[track_caller]
        pub fn immediately_unbond_node(&mut self, node: impl Into<NodeQueryType>) {
            let node_id = self.get_node_id(node);
            let env = self.env();
            pending_events::unbond_nym_node(self.deps_mut(), &env, env.block.height, node_id)
                .unwrap();
        }

        pub fn immediately_unbond_mixnode(&mut self, mix_id: NodeId) {
            let env = self.env();
            pending_events::unbond_mixnode(self.deps_mut(), &env, env.block.height, mix_id)
                .unwrap();
        }

        pub fn immediately_unbond_nymnode(&mut self, node_id: NodeId) {
            let env = self.env();
            pending_events::unbond_nym_node(self.deps_mut(), &env, env.block.height, node_id)
                .unwrap();
        }

        pub fn add_immediate_delegation(
            &mut self,
            delegator: &str,
            amount: impl Into<Uint128>,
            target: NodeId,
        ) {
            let denom = rewarding_denom(self.deps().storage).unwrap();
            let amount = Coin {
                denom,
                amount: amount.into(),
            };
            let env = self.env();
            pending_events::delegate(
                self.deps_mut(),
                &env,
                env.block.height,
                Addr::unchecked(delegator),
                target,
                amount,
            )
            .unwrap();
        }

        #[allow(unused)]
        pub fn add_delegation(
            &mut self,
            delegator: &str,
            amount: impl Into<Uint128>,
            target: NodeId,
        ) {
            let denom = rewarding_denom(self.deps().storage).unwrap();
            let amount = Coin {
                denom,
                amount: amount.into(),
            };
            let env = self.env();
            delegate(self.deps_mut(), env, delegator, vec![amount], target)
        }

        pub fn remove_immediate_delegation(&mut self, delegator: &str, target: NodeId) {
            let height = self.env.block.height;
            pending_events::undelegate(self.deps_mut(), height, Addr::unchecked(delegator), target)
                .unwrap();
        }

        pub fn start_epoch_transition(&mut self) {
            let env = self.env.clone();
            let sender = self.rewarding_validator.clone();
            try_begin_epoch_transition(self.deps_mut(), env, sender).unwrap();
        }

        pub fn epoch_state(&self) -> EpochState {
            interval_storage::current_epoch_status(self.deps().storage)
                .unwrap()
                .state
        }

        pub fn set_epoch_in_progress_state(&mut self) {
            let being_advanced_by = self.rewarding_validator.sender.clone();
            interval_storage::save_current_epoch_status(
                self.deps_mut().storage,
                &EpochStatus {
                    being_advanced_by,
                    state: EpochState::InProgress,
                },
            )
            .unwrap();
        }

        pub fn set_epoch_reconciliation_state(&mut self) {
            let being_advanced_by = self.rewarding_validator.sender.clone();
            interval_storage::save_current_epoch_status(
                self.deps_mut().storage,
                &EpochStatus {
                    being_advanced_by,
                    state: EpochState::ReconcilingEvents,
                },
            )
            .unwrap();
        }

        pub fn set_epoch_role_assignment_state(&mut self) {
            let being_advanced_by = self.rewarding_validator.sender.clone();
            interval_storage::save_current_epoch_status(
                self.deps_mut().storage,
                &EpochStatus {
                    being_advanced_by,
                    state: EpochState::RoleAssignment {
                        next: Role::first(),
                    },
                },
            )
            .unwrap();
        }

        #[allow(unused)]
        pub fn pending_operator_reward(&mut self, mix: NodeId) -> Decimal {
            query_pending_mixnode_operator_reward(self.deps(), mix)
                .unwrap()
                .amount_earned_detailed
                .expect("no reward!")
        }

        #[allow(unused)]
        pub fn pending_delegator_reward(&mut self, delegator: &str, target: NodeId) -> Decimal {
            query_pending_delegator_reward(self.deps(), delegator.into(), target, None)
                .unwrap()
                .amount_earned_detailed
                .expect("no reward!")
        }

        pub fn skip_to_next_epoch_end(&mut self) {
            self.skip_to_next_epoch();
            self.skip_to_current_epoch_end();
        }

        pub fn skip_to_current_epoch_end(&mut self) {
            let interval = interval_storage::current_interval(self.deps().storage).unwrap();
            let epoch_end = interval.current_epoch_end_unix_timestamp();
            // skip few blocks just in case
            self.env.block.height += 10;
            self.env.block.time = Timestamp::from_seconds(epoch_end as u64);
        }

        pub fn skip_to_current_interval_end(&mut self) {
            let interval = interval_storage::current_interval(self.deps().storage).unwrap();
            let interval_end = interval.current_interval_end_unix_timestamp();
            // skip few blocks just in case
            self.env.block.height += 10;
            self.env.block.time = Timestamp::from_seconds(interval_end as u64);
        }

        pub fn skip_to_next_epoch(&mut self) {
            let interval = interval_storage::current_interval(self.deps().storage).unwrap();
            let epoch_end = interval.current_epoch_end_unix_timestamp();
            // skip few blocks just in case
            self.env.block.height += 10;
            self.env.block.time = Timestamp::from_seconds(epoch_end as u64 + 1);
            let advanced = interval.advance_epoch();

            assert_eq!(
                interval.current_epoch_absolute_id() + 1,
                advanced.current_epoch_absolute_id()
            );

            if interval.current_epoch_id() != interval.epochs_in_interval() - 1 {
                assert_eq!(interval.current_epoch_id() + 1, advanced.current_epoch_id())
            } else {
                assert_eq!(advanced.current_epoch_id(), 0);
                assert_eq!(
                    interval.current_interval_id() + 1,
                    advanced.current_interval_id()
                )
            }

            interval_storage::save_interval(self.deps_mut().storage, &advanced).unwrap();
            // if we're going into next epoch, we're back into in progress
            self.set_epoch_in_progress_state();
        }

        pub fn reset_role_assignment(&mut self) {
            let active_bucket = ACTIVE_ROLES_BUCKET.load(&self.deps.storage).unwrap();

            for role in vec![
                Role::EntryGateway,
                Role::ExitGateway,
                Role::Layer1,
                Role::Layer2,
                Role::Layer3,
                Role::Standby,
            ] {
                ROLES
                    .save(&mut self.deps.storage, (active_bucket as u8, role), &vec![])
                    .unwrap();
            }
        }

        // note: this does NOT assign gateway role
        pub fn force_change_rewarded_set(&mut self, nodes: Vec<NodeId>) {
            // reset current assignment
            self.reset_role_assignment();
            let mut roles = HashMap::new();
            for node in nodes {
                let role = self.immediately_assign_lowest_mix_layer(node);
                let assigned = roles.entry(role).or_insert(Vec::new());
                assigned.push(node)
            }

            // we cheat a bit to write to the 'active' bucket instead
            swap_active_role_bucket(self.deps_mut().storage).unwrap();

            for (role, assignment) in roles {
                save_assignment(
                    self.deps_mut().storage,
                    RoleAssignment {
                        role,
                        nodes: assignment,
                    },
                )
                .unwrap();
            }

            swap_active_role_bucket(self.deps_mut().storage).unwrap();
        }

        pub fn instantiate_simulator(&self, node: NodeId) -> Simulator {
            simulator_from_single_node_state(self.deps(), node)
        }

        pub fn execute_all_pending_events(&mut self) {
            let env = self.env();
            execute_all_pending_events(self.deps_mut(), env)
        }

        pub fn pending_interval_events(&self) -> Vec<PendingIntervalEventData> {
            interval_storage::PENDING_INTERVAL_EVENTS
                .range(self.deps().storage, None, None, Order::Ascending)
                .map(|res| res.unwrap().1)
                .collect::<Vec<_>>()
        }

        pub fn pending_epoch_events(&self) -> Vec<PendingEpochEventData> {
            interval_storage::PENDING_EPOCH_EVENTS
                .range(self.deps().storage, None, None, Order::Ascending)
                .map(|res| res.unwrap().1)
                .collect::<Vec<_>>()
        }

        pub fn active_node_work(&self) -> WorkFactor {
            self.rewarding_params().active_node_work()
        }

        #[allow(dead_code)]
        pub fn standby_node_work(&self) -> WorkFactor {
            self.rewarding_params().standby_node_work()
        }

        pub fn active_node_params(&self, performance: f32) -> NodeRewardingParameters {
            NodeRewardingParameters {
                performance: test_helpers::performance(performance),
                work_factor: self.active_node_work(),
            }
        }

        #[allow(dead_code)]
        pub fn standby_node_params(&self, performance: f32) -> NodeRewardingParameters {
            NodeRewardingParameters {
                performance: test_helpers::performance(performance),
                work_factor: self.standby_node_work(),
            }
        }

        #[track_caller]
        pub fn reward_with_distribution_ignore_state(
            &mut self,
            node_id: NodeId,
            params: NodeRewardingParameters,
        ) -> RewardDistribution {
            self.reward_with_distribution_with_state_bypass(
                node_id,
                params.performance,
                params.work_factor,
            )
        }

        #[track_caller]
        pub fn reward_with_distribution_with_state_bypass(
            &mut self,
            node_id: NodeId,
            performance: Performance,
            work_factor: WorkFactor,
        ) -> RewardDistribution {
            let initial_status =
                interval_storage::current_epoch_status(self.deps().storage).unwrap();
            self.start_epoch_transition();
            let res = self.reward_with_distribution(
                node_id,
                NodeRewardingParameters::new(performance, work_factor),
            );
            interval_storage::save_current_epoch_status(self.deps_mut().storage, &initial_status)
                .unwrap();
            res
        }

        #[allow(dead_code)]
        #[track_caller]
        pub fn node_role(&self, node_id: NodeId) -> Role {
            if read_assigned_roles(&self.deps.storage, Role::EntryGateway)
                .unwrap()
                .contains(&node_id)
            {
                Role::EntryGateway
            } else if read_assigned_roles(&self.deps.storage, Role::ExitGateway)
                .unwrap()
                .contains(&node_id)
            {
                Role::ExitGateway
            } else if read_assigned_roles(&self.deps.storage, Role::Layer1)
                .unwrap()
                .contains(&node_id)
            {
                Role::Layer1
            } else if read_assigned_roles(&self.deps.storage, Role::Layer2)
                .unwrap()
                .contains(&node_id)
            {
                Role::Layer2
            } else if read_assigned_roles(&self.deps.storage, Role::Layer3)
                .unwrap()
                .contains(&node_id)
            {
                Role::Layer3
            } else if read_assigned_roles(&self.deps.storage, Role::Standby)
                .unwrap()
                .contains(&node_id)
            {
                Role::Standby
            } else {
                let caller = std::panic::Location::caller();
                panic!("{caller}: no assigned roles")
            }
        }

        pub fn legacy_rewarding_params(
            &self,
            node_id: NodeId,
            performance: f32,
        ) -> NodeRewardingParameters {
            let performance = test_helpers::performance(performance);
            let work_factor = self.get_legacy_rewarding_node_work_factor(node_id);
            NodeRewardingParameters {
                performance,
                work_factor,
            }
        }

        pub fn get_legacy_rewarding_node_work_factor(&self, node_id: NodeId) -> Decimal {
            let global_rewarding_params = self.rewarding_params();
            let work_factor =
                match expensive_role_lookup(self.deps.as_ref().storage, node_id).unwrap() {
                    None => Decimal::zero(),
                    Some(Role::Standby) => global_rewarding_params.standby_node_work(),
                    _ => global_rewarding_params.active_node_work(),
                };
            work_factor
        }

        #[track_caller]
        pub fn reward_with_distribution(
            &mut self,
            node_id: NodeId,
            rewarding_params: NodeRewardingParameters,
        ) -> RewardDistribution {
            let env = self.env();
            let sender = self.rewarding_validator();

            let res =
                try_reward_node(self.deps_mut(), env, sender, node_id, rewarding_params).unwrap();
            let operator: Decimal = find_attribute(
                Some(MixnetEventType::NodeRewarding.to_string()),
                OPERATOR_REWARD_KEY,
                &res,
            )
            .parse()
            .unwrap();
            let delegates: Decimal = find_attribute(
                Some(MixnetEventType::NodeRewarding.to_string()),
                DELEGATES_REWARD_KEY,
                &res,
            )
            .parse()
            .unwrap();

            RewardDistribution {
                operator,
                delegates,
            }
        }

        pub fn read_delegation(
            &mut self,
            mix: NodeId,
            owner: &str,
            proxy: Option<&str>,
        ) -> Delegation {
            read_delegation(
                self.deps().storage,
                mix,
                &Addr::unchecked(owner),
                &proxy.map(Addr::unchecked),
            )
            .unwrap()
        }

        pub fn mix_rewarding(&self, node: NodeId) -> NodeRewarding {
            rewards_storage::MIXNODE_REWARDING
                .load(self.deps().storage, node)
                .unwrap()
        }

        #[allow(unused)]
        pub fn mix_bond(&self, mix_id: NodeId) -> MixNodeBond {
            mixnode_bonds().load(self.deps().storage, mix_id).unwrap()
        }

        #[track_caller]
        pub fn delegation(&self, mix: NodeId, owner: &str, proxy: &Option<Addr>) -> Delegation {
            let caller = std::panic::Location::caller();

            read_delegation(self.deps().storage, mix, &Addr::unchecked(owner), proxy)
                .unwrap_or_else(|| {
                    panic!("{caller} failed with: delegation for {mix}/{owner} doesn't exist")
                })
        }
    }

    pub fn ed25519_sign_message<T: Serialize + SigningPurpose>(
        message: SignableMessage<T>,
        private_key: &identity::PrivateKey,
    ) -> MessageSignature {
        match message.algorithm {
            SigningAlgorithm::Ed25519 => {
                let plaintext = message.to_plaintext().unwrap();
                let signature = private_key.sign(plaintext);
                MessageSignature::from(signature.to_bytes().as_ref())
            }
            SigningAlgorithm::Secp256k1 => {
                unimplemented!()
            }
        }
    }

    pub fn simulator_from_single_node_state(deps: Deps<'_>, node: NodeId) -> Simulator {
        let mix_rewarding = rewards_storage::MIXNODE_REWARDING
            .load(deps.storage, node)
            .unwrap();
        let delegations = query_node_delegations_paged(deps, node, None, None).unwrap();
        if delegations.delegations.len() as u32
            == constants::DELEGATION_PAGE_DEFAULT_RETRIEVAL_LIMIT
        {
            // can't be bothered to deal with paging for this test case since it's incredibly unlikely
            // we'd ever need it
            panic!("too many delegations")
        }
        let rewarding_params = rewards_storage::REWARDING_PARAMS
            .load(deps.storage)
            .unwrap();
        let interval = interval_storage::current_interval(deps.storage).unwrap();
        let mut simulator = Simulator::new(rewarding_params, interval);
        simulator.nodes.insert(
            0,
            SimulatedNode {
                mix_id: 0,
                rewarding_details: mix_rewarding,
                delegations: delegations
                    .delegations
                    .into_iter()
                    .map(|d| (d.owner.to_string(), d))
                    .collect(),
            },
        );

        simulator
    }

    pub fn get_bank_send_msg(response: &Response) -> Option<(String, Vec<Coin>)> {
        for msg in &response.messages {
            if let CosmosMsg::Bank(BankMsg::Send { to_address, amount }) = &msg.msg {
                return Some((to_address.clone(), amount.clone()));
            }
        }
        None
    }

    pub fn find_attribute<S: Into<String>>(
        event_type: Option<S>,
        attribute: &str,
        response: &Response,
    ) -> String {
        let event_type = event_type.map(Into::into);
        for event in &response.events {
            if let Some(typ) = &event_type {
                if &event.ty != typ {
                    continue;
                }
            }
            if let Some(attr) = may_find_attribute(event, attribute) {
                return attr;
            }
        }
        // this is only used in tests so panic here is fine
        panic!("did not find the attribute")
    }

    pub(crate) trait FindAttribute {
        fn attribute<E, S>(&self, event_type: E, attribute: &str) -> String
        where
            E: Into<Option<S>>,
            S: Into<String>;

        fn parsed_attribute<E, S, T>(&self, event_type: E, attribute: &str) -> T
        where
            E: Into<Option<S>>,
            S: Into<String>,
            T: FromStr,
            <T as FromStr>::Err: Debug;

        fn decimal<E, S>(&self, event_type: E, attribute: &str) -> Decimal
        where
            E: Into<Option<S>>,
            S: Into<String>,
        {
            self.parsed_attribute(event_type, attribute)
        }
    }

    impl FindAttribute for Response {
        fn attribute<E, S>(&self, event_type: E, attribute: &str) -> String
        where
            E: Into<Option<S>>,
            S: Into<String>,
        {
            find_attribute(event_type.into(), attribute, self)
        }

        fn parsed_attribute<E, S, T>(&self, event_type: E, attribute: &str) -> T
        where
            E: Into<Option<S>>,
            S: Into<String>,
            T: FromStr,
            <T as FromStr>::Err: Debug,
        {
            find_attribute(event_type.into(), attribute, self)
                .parse()
                .unwrap()
        }
    }

    // using floats in tests is fine
    // (what it does is converting % value, like 12.34 into `Performance` (`Percent`)
    // which internally is represented by decimal `0.1234`
    pub fn performance(val: f32) -> Performance {
        assert!(val <= 100.0);
        assert!(val >= 0.0);

        // hehe, that's such a nasty conversion, but it works for test purposes
        let str = (val / 100.0).to_string();
        let dec = str.parse().unwrap();
        Performance::new(dec).unwrap()
    }

    // use rng with constant seed for all tests so that they would be deterministic
    pub fn test_rng() -> ChaCha20Rng {
        let dummy_seed = [42u8; 32];
        rand_chacha::ChaCha20Rng::from_seed(dummy_seed)
    }

    pub fn execute_all_pending_events(mut deps: DepsMut<'_>, env: Env) {
        perform_pending_epoch_actions(deps.branch(), &env, None).unwrap();
        perform_pending_interval_actions(deps.branch(), &env, None).unwrap();
    }

    // pub fn mixnode_with_signature(
    //     mut rng: impl RngCore + CryptoRng,
    //     deps: Deps<'_>,
    //     sender: &str,
    //     stake: Option<Vec<Coin>>,
    // ) -> (MixNode, MessageSignature, KeyPair) {
    //     // hehe stupid workaround for bypassing the immutable borrow and removing duplicate code
    //
    //     let stake = stake.unwrap_or(good_mixnode_pledge());
    //
    //     let keypair = identity::KeyPair::new(&mut rng);
    //     let identity_key = keypair.public_key().to_base58_string();
    //     let legit_sphinx_keys = nym_crypto::asymmetric::encryption::KeyPair::new(&mut rng);
    //
    //     let mixnode = MixNode {
    //         identity_key,
    //         sphinx_key: legit_sphinx_keys.public_key().to_base58_string(),
    //         ..tests::fixtures::mix_node_fixture()
    //     };
    //     let msg = mixnode_bonding_sign_payload(deps, sender, None, mixnode.clone(), stake.clone());
    //     let owner_signature = ed25519_sign_message(msg, keypair.private_key());
    //
    //     (mixnode, owner_signature, keypair)
    // }

    // pub fn gateway_with_signature(
    //     mut rng: impl RngCore + CryptoRng,
    //     deps: Deps<'_>,
    //     sender: &str,
    //     stake: Option<Vec<Coin>>,
    // ) -> (Gateway, MessageSignature) {
    //     let stake = stake.unwrap_or(good_gateway_pledge());
    //
    //     let keypair = identity::KeyPair::new(&mut rng);
    //     let identity_key = keypair.public_key().to_base58_string();
    //     let legit_sphinx_keys = nym_crypto::asymmetric::encryption::KeyPair::new(&mut rng);
    //
    //     let gateway = Gateway {
    //         identity_key,
    //         sphinx_key: legit_sphinx_keys.public_key().to_base58_string(),
    //         ..tests::fixtures::gateway_fixture()
    //     };
    //
    //     let msg = gateway_bonding_sign_payload(deps, sender, None, gateway.clone(), stake.clone());
    //     let owner_signature = ed25519_sign_message(msg, keypair.private_key());
    //
    //     (gateway, owner_signature)
    // }

    pub fn add_dummy_delegations(mut deps: DepsMut<'_>, env: Env, mix_id: NodeId, n: usize) {
        for i in 0..n {
            pending_events::delegate(
                deps.branch(),
                &env,
                env.block.height,
                Addr::unchecked(format!("owner{}", i)),
                mix_id,
                tests::fixtures::good_mixnode_pledge().pop().unwrap(),
            )
            .unwrap();
        }
    }

    // pub fn add_dummy_mixnodes(
    //     mut rng: impl RngCore + CryptoRng,
    //     mut deps: DepsMut<'_>,
    //     env: Env,
    //     n: usize,
    // ) {
    //     for i in 0..n {
    //         add_mixnode(
    //             &mut rng,
    //             deps.branch(),
    //             env.clone(),
    //             &format!("owner{}", i),
    //             tests::fixtures::good_mixnode_pledge(),
    //         );
    //     }
    // }

    pub fn add_dummy_unbonded_mixnodes(
        mut rng: impl RngCore + CryptoRng,
        mut deps: DepsMut<'_>,
        n: usize,
    ) {
        for i in 0..n {
            add_unbonded_mixnode(&mut rng, deps.branch(), None, &format!("owner{}", i));
        }
    }

    pub fn add_dummy_unbonded_mixnodes_with_owner(
        mut rng: impl RngCore + CryptoRng,
        mut deps: DepsMut<'_>,
        owner: &str,
        n: usize,
    ) {
        for _ in 0..n {
            add_unbonded_mixnode(&mut rng, deps.branch(), None, owner);
        }
    }

    pub fn add_dummy_unbonded_mixnodes_with_identity(
        mut rng: impl RngCore + CryptoRng,
        mut deps: DepsMut<'_>,
        identity: &str,
        n: usize,
    ) {
        for i in 0..n {
            add_unbonded_mixnode(
                &mut rng,
                deps.branch(),
                Some(identity),
                &format!("owner{}", i),
            );
        }
    }

    // same note as with `add_mixnode`
    pub fn add_unbonded_mixnode(
        mut rng: impl RngCore + CryptoRng,
        deps: DepsMut<'_>,
        identity_key: Option<&str>,
        owner: &str,
    ) -> NodeId {
        let id = loop {
            let candidate = rng.next_u32();
            if !mixnodes_storage::unbonded_mixnodes().has(deps.storage, candidate) {
                break candidate;
            }
        };

        // we don't care about 'correctness' of the identity key here
        mixnodes_storage::unbonded_mixnodes()
            .save(
                deps.storage,
                id,
                &UnbondedMixnode {
                    identity_key: identity_key
                        .unwrap_or(&*format!("identity{}", id))
                        .to_string(),
                    owner: Addr::unchecked(owner),
                    proxy: None,
                    unbonding_height: 12345,
                },
            )
            .unwrap();

        id
    }

    pub fn nymnode_bonding_sign_payload(
        deps: Deps<'_>,
        owner: &str,
        node: NymNode,
        stake: Vec<Coin>,
    ) -> SignableNymNodeBondingMsg {
        let cost_params = tests::fixtures::node_cost_params_fixture();
        let nonce =
            signing_storage::get_signing_nonce(deps.storage, Addr::unchecked(owner)).unwrap();

        let payload = NymNodeBondingPayload::new(node, cost_params);
        let content = ContractMessageContent::new(Addr::unchecked(owner), stake, payload);
        SignableNymNodeBondingMsg::new(nonce, content)
    }

    pub fn mixnode_bonding_sign_payload(
        deps: Deps<'_>,
        owner: &str,
        mixnode: MixNode,
        stake: Vec<Coin>,
    ) -> SignableMixNodeBondingMsg {
        let cost_params = tests::fixtures::node_cost_params_fixture();
        let nonce =
            signing_storage::get_signing_nonce(deps.storage, Addr::unchecked(owner)).unwrap();

        let payload = MixnodeBondingPayload::new(mixnode, cost_params);
        let content = ContractMessageContent::new(Addr::unchecked(owner), stake, payload);
        SignableMixNodeBondingMsg::new(nonce, content)
    }

    pub fn gateway_bonding_sign_payload(
        deps: Deps<'_>,
        owner: &str,
        gateway: Gateway,
        stake: Vec<Coin>,
    ) -> SignableGatewayBondingMsg {
        let nonce =
            signing_storage::get_signing_nonce(deps.storage, Addr::unchecked(owner)).unwrap();

        let payload = GatewayBondingPayload::new(gateway);
        let content = ContractMessageContent::new(Addr::unchecked(owner), stake, payload);
        SignableGatewayBondingMsg::new(nonce, content)
    }

    fn intial_rewarded_set_params() -> RewardedSetParams {
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
            rewarded_set_params: intial_rewarded_set_params(),
        }
    }

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            rewarding_validator_address: "rewarder".into(),
            vesting_contract_address: "vesting-contract".to_string(),
            rewarding_denom: TEST_COIN_DENOM.to_string(),
            epochs_in_interval: 720,
            epoch_duration: Duration::from_secs(60 * 60),
            initial_rewarding_params: initial_rewarding_params(),
            profit_margin: Default::default(),
            interval_operating_cost: Default::default(),
        };
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }

    pub fn delegate(deps: DepsMut<'_>, env: Env, sender: &str, stake: Vec<Coin>, mix_id: NodeId) {
        let info = mock_info(sender, &stake);
        try_delegate_to_node(deps, env, info, mix_id).unwrap();
    }

    pub(crate) fn read_delegation(
        storage: &dyn Storage,
        mix: NodeId,
        owner: &Addr,
        proxy: &Option<Addr>,
    ) -> Option<Delegation> {
        delegations_storage::delegations()
            .may_load(
                storage,
                Delegation::generate_storage_key(mix, owner, proxy.as_ref()),
            )
            .unwrap()
    }
}
