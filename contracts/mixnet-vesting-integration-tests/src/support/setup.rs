// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::fixtures;
use crate::support::helpers::{
    mixnet_contract_wrapper, rewarding_validator, test_rng, vesting_contract_wrapper,
};
use cosmwasm_std::{coins, Addr, Coin, Decimal, Timestamp};
use cw_multi_test::{App, AppBuilder, Executor};
use nym_contracts_common::signing::{ContractMessageContent, MessageSignature, Nonce};
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::nym_node::{EpochAssignmentResponse, Role, RolesMetadataResponse};
use nym_mixnet_contract_common::reward_params::{NodeRewardingParameters, Performance};
use nym_mixnet_contract_common::{
    CurrentIntervalResponse, MixnodeBondingPayload, NodeCostParams, RewardedSet, RewardingParams,
    RoleAssignment, SignableMixNodeBondingMsg,
};
use nym_mixnet_contract_common::{
    ExecuteMsg as MixnetExecuteMsg, MixNode, QueryMsg as MixnetQueryMsg,
};
use rand_chacha::ChaCha20Rng;
use std::collections::HashMap;

// our global accounts that should always get some coins at the start
pub const MIXNET_OWNER: &str = "mixnet-owner";
pub const VESTING_OWNER: &str = "vesting-owner";
pub const REWARDING_VALIDATOR: &str = "rewarding-validator";
pub const MIX_DENOM: &str = "unym";

#[allow(unused)]
pub struct ContractInstantiationResult {
    mixnet_contract_address: Addr,
    vesting_contract_address: Addr,
}

#[allow(unused)]
pub struct TestSetupBuilder {
    mixnet_init_msg: nym_mixnet_contract_common::InstantiateMsg,
    initial_balances: HashMap<Addr, Vec<Coin>>,
}

#[allow(unused)]
impl TestSetupBuilder {
    pub fn new() -> Self {
        TestSetupBuilder {
            mixnet_init_msg: fixtures::default_mixnet_init_msg(),
            initial_balances: Default::default(),
        }
    }

    pub fn with_mixnet_init_msg(
        mut self,
        mixnet_init_msg: nym_mixnet_contract_common::InstantiateMsg,
    ) -> Self {
        self.mixnet_init_msg = mixnet_init_msg;
        self
    }

    pub fn with_initial_balances(mut self, initial_balances: HashMap<Addr, Vec<Coin>>) -> Self {
        self.initial_balances = initial_balances;
        self
    }

    pub fn with_initial_balance(mut self, addr: impl Into<String>, balance: Vec<Coin>) -> Self {
        self.initial_balances.insert(Addr::unchecked(addr), balance);
        self
    }

    pub fn build(self) -> TestSetup {
        TestSetup::new(self.initial_balances, self.mixnet_init_msg)
    }
}

#[allow(unused)]
pub struct TestSetup {
    pub app: App,
    pub rng: ChaCha20Rng,

    pub mixnet_contract: Addr,
}

#[allow(unused)]
impl TestSetup {
    pub fn new_simple() -> Self {
        TestSetup::new(Default::default(), fixtures::default_mixnet_init_msg())
    }

    pub fn new(
        initial_balances: HashMap<Addr, Vec<Coin>>,
        custom_mixnet_init: nym_mixnet_contract_common::InstantiateMsg,
    ) -> Self {
        let (app, contracts) = instantiate_contracts(initial_balances, Some(custom_mixnet_init));
        TestSetup {
            app,
            rng: test_rng(),
            mixnet_contract: contracts.mixnet_contract_address,
        }
    }

    pub fn mixnet_contract(&self) -> Addr {
        self.mixnet_contract.clone()
    }

    pub fn skip_to_current_epoch_end(&mut self) {
        let current_interval: CurrentIntervalResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetCurrentIntervalDetails {},
            )
            .unwrap();
        let epoch_end = current_interval.interval.current_epoch_end_unix_timestamp();

        self.app.update_block(|current_block| {
            // skip few blocks just in case
            current_block.height += 10;
            current_block.time = Timestamp::from_seconds(epoch_end as u64)
        })
    }

    fn get_rewarded_set(&self) -> RewardedSet {
        let metadata: RolesMetadataResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRewardedSetMetadata {},
            )
            .unwrap();

        let entry: EpochAssignmentResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRoleAssignment {
                    role: Role::EntryGateway,
                },
            )
            .unwrap();
        assert_eq!(entry.epoch_id, metadata.metadata.epoch_id);

        let exit: EpochAssignmentResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRoleAssignment {
                    role: Role::ExitGateway,
                },
            )
            .unwrap();
        assert_eq!(exit.epoch_id, metadata.metadata.epoch_id);

        let layer1: EpochAssignmentResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRoleAssignment { role: Role::Layer1 },
            )
            .unwrap();
        assert_eq!(layer1.epoch_id, metadata.metadata.epoch_id);

        let layer2: EpochAssignmentResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRoleAssignment { role: Role::Layer2 },
            )
            .unwrap();
        assert_eq!(layer2.epoch_id, metadata.metadata.epoch_id);

        let layer3: EpochAssignmentResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRoleAssignment { role: Role::Layer3 },
            )
            .unwrap();
        assert_eq!(layer3.epoch_id, metadata.metadata.epoch_id);

        let standby: EpochAssignmentResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRoleAssignment {
                    role: Role::Standby,
                },
            )
            .unwrap();
        assert_eq!(standby.epoch_id, metadata.metadata.epoch_id);

        RewardedSet {
            entry_gateways: entry.nodes,
            exit_gateways: exit.nodes,
            layer1: layer1.nodes,
            layer2: layer2.nodes,
            layer3: layer3.nodes,
            standby: standby.nodes,
        }
    }

    pub fn full_mixnet_epoch_operations(&mut self) {
        let rewarded_set = self.get_rewarded_set();

        let current_params: RewardingParams = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRewardingParams {},
            )
            .unwrap();
        // TODO: handle paging

        // begin epoch transition
        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::BeginEpochTransition {},
                &[],
            )
            .unwrap();

        let work =
            Decimal::one() / Decimal::from_ratio(rewarded_set.rewarded_set_size() as u64, 1u64);
        let params = NodeRewardingParameters::new(Performance::hundred(), work);

        let mut nodes = rewarded_set
            .layer1
            .iter()
            .chain(rewarded_set.layer2.iter())
            .chain(rewarded_set.layer3.iter())
            .chain(rewarded_set.entry_gateways.iter())
            .chain(rewarded_set.exit_gateways.iter())
            .chain(rewarded_set.standby.iter())
            .copied()
            .collect::<Vec<_>>();

        nodes.sort();

        // reward
        for (node_id) in nodes {
            self.app
                .execute_contract(
                    rewarding_validator(),
                    self.mixnet_contract(),
                    &MixnetExecuteMsg::RewardNode { node_id, params },
                    &[],
                )
                .unwrap();
        }

        // events
        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::ReconcileEpochEvents { limit: None },
                &[],
            )
            .unwrap();

        // don't bother changing the active set, use the same node for update and advance

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::EntryGateway,
                        nodes: rewarded_set.entry_gateways,
                    },
                },
                &[],
            )
            .unwrap();

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::ExitGateway,
                        nodes: rewarded_set.exit_gateways,
                    },
                },
                &[],
            )
            .unwrap();

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Layer1,
                        nodes: rewarded_set.layer1,
                    },
                },
                &[],
            )
            .unwrap();

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Layer2,
                        nodes: rewarded_set.layer2,
                    },
                },
                &[],
            )
            .unwrap();

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Layer3,
                        nodes: rewarded_set.layer3,
                    },
                },
                &[],
            )
            .unwrap();

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AssignRoles {
                    assignment: RoleAssignment {
                        role: Role::Standby,
                        nodes: rewarded_set.standby,
                    },
                },
                &[],
            )
            .unwrap();
    }

    pub fn advance_mixnet_epoch(&mut self) {
        self.skip_to_current_epoch_end();
        self.full_mixnet_epoch_operations();
    }

    pub fn valid_mixnode_with_sig(
        &mut self,
        owner: &str,
        cost_params: NodeCostParams,
        stake: Coin,
    ) -> (MixNode, MessageSignature) {
        let signing_nonce: Nonce = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetSigningNonce {
                    address: owner.to_string(),
                },
            )
            .unwrap();

        let keypair = identity::KeyPair::new(&mut self.rng);
        let identity_key = keypair.public_key().to_base58_string();
        let legit_sphinx_keys = nym_crypto::asymmetric::encryption::KeyPair::new(&mut self.rng);

        let mixnode = MixNode {
            identity_key,
            sphinx_key: legit_sphinx_keys.public_key().to_base58_string(),
            host: "mix.node.org".to_string(),
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8000,
            version: "1.1.14".to_string(),
        };

        let payload = MixnodeBondingPayload::new(mixnode.clone(), cost_params);
        let content = ContractMessageContent::new(Addr::unchecked(owner), vec![stake], payload);
        let sign_payload = SignableMixNodeBondingMsg::new(signing_nonce, content);
        let plaintext = sign_payload.to_plaintext().unwrap();
        let signature = keypair.private_key().sign(plaintext);
        let msg_signature = MessageSignature::from(signature.to_bytes().as_ref());

        (mixnode, msg_signature)
    }
}

pub fn instantiate_contracts(
    mut initial_funds: HashMap<Addr, Vec<Coin>>,
    custom_mixnet_init: Option<nym_mixnet_contract_common::InstantiateMsg>,
) -> (App, ContractInstantiationResult) {
    // add our global addresses to the map
    initial_funds.insert(
        Addr::unchecked(MIXNET_OWNER),
        coins(100_000_000_000, MIX_DENOM),
    );

    initial_funds.insert(
        Addr::unchecked(VESTING_OWNER),
        coins(100_000_000_000, MIX_DENOM),
    );

    initial_funds.insert(
        Addr::unchecked(REWARDING_VALIDATOR),
        coins(1_000_000_000_000, MIX_DENOM),
    );

    let mut app = AppBuilder::new().build(|router, _api, storage| {
        for (addr, funds) in initial_funds {
            router
                .bank
                .init_balance(storage, &addr, funds.clone())
                .unwrap()
        }
    });

    let mixnet_code_id = app.store_code(mixnet_contract_wrapper());
    let vesting_code_id = app.store_code(vesting_contract_wrapper());

    let mixnet_contract_address = app
        .instantiate_contract(
            mixnet_code_id,
            Addr::unchecked(MIXNET_OWNER),
            &custom_mixnet_init.unwrap_or(fixtures::default_mixnet_init_msg()),
            &[],
            "mixnet-contract",
            Some(MIXNET_OWNER.to_string()),
        )
        .unwrap();

    let vesting_contract_address = app
        .instantiate_contract(
            vesting_code_id,
            Addr::unchecked(VESTING_OWNER),
            &nym_vesting_contract_common::InitMsg {
                mixnet_contract_address: mixnet_contract_address.to_string(),
                mix_denom: MIX_DENOM.to_string(),
            },
            &[],
            "vesting-contract",
            Some(VESTING_OWNER.to_string()),
        )
        .unwrap();

    // now fix up vesting contract address...
    app.migrate_contract(
        Addr::unchecked(MIXNET_OWNER),
        mixnet_contract_address.clone(),
        &nym_mixnet_contract_common::MigrateMsg {
            vesting_contract_address: Some(vesting_contract_address.to_string()),
        },
        mixnet_code_id,
    )
    .unwrap();

    (
        app,
        ContractInstantiationResult {
            mixnet_contract_address,
            vesting_contract_address,
        },
    )
}
