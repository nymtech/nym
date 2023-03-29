// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{coin, coins, Addr, Coin, Decimal, Empty, Timestamp};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};
use nym_contracts_common::signing::{ContractMessageContent, MessageSignature, Nonce};
use nym_crypto::asymmetric::identity;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{
    CurrentIntervalResponse, InitialRewardingParams, LayerAssignment, MixNodeCostParams,
    MixnodeBondingPayload, PagedRewardedSetResponse, Percent, RewardingParams,
    SignableMixNodeBondingMsg,
};
use nym_mixnet_contract_common::{
    ExecuteMsg as MixnetExecuteMsg, MixNode, QueryMsg as MixnetQueryMsg,
};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::collections::HashMap;
use std::time::Duration;

// our global accounts that should always get some coins at the start
pub const MIXNET_OWNER: &str = "mixnet-owner";
pub const VESTING_OWNER: &str = "vesting-owner";
pub const REWARDING_VALIDATOR: &str = "rewarding-validator";
pub const MIX_DENOM: &str = "unym";

pub struct ContractInstantiationResult {
    mixnet_contract_address: Addr,
    vesting_contract_address: Addr,
}

#[allow(unused)]
pub fn mixnet_owner() -> Addr {
    Addr::unchecked(MIXNET_OWNER)
}

pub fn vesting_owner() -> Addr {
    Addr::unchecked(VESTING_OWNER)
}

pub fn rewarding_validator() -> Addr {
    Addr::unchecked(REWARDING_VALIDATOR)
}

pub fn mix_coins(amount: u128) -> Vec<Coin> {
    coins(amount.into(), MIX_DENOM)
}

pub fn mix_coin(amount: u128) -> Coin {
    coin(amount, MIX_DENOM)
}

pub fn test_rng() -> ChaCha20Rng {
    let dummy_seed = [42u8; 32];
    ChaCha20Rng::from_seed(dummy_seed)
}

pub struct TestSetup {
    pub app: App,
    pub rng: ChaCha20Rng,

    pub mixnet_contract: Addr,
    pub vesting_contract: Addr,
}

impl TestSetup {
    pub fn new() -> Self {
        TestSetup::new_with_balances(Default::default())
    }

    pub fn new_with_balances(balances: HashMap<Addr, Vec<Coin>>) -> Self {
        let (app, contracts) = instantiate_contracts(balances);
        TestSetup {
            app,
            rng: test_rng(),
            mixnet_contract: contracts.mixnet_contract_address,
            vesting_contract: contracts.vesting_contract_address,
        }
    }

    pub fn mixnet_contract(&self) -> Addr {
        self.mixnet_contract.clone()
    }

    pub fn vesting_contract(&self) -> Addr {
        self.vesting_contract.clone()
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

    pub fn full_mixnet_epoch_operations(&mut self) {
        let current_rewarded_set: PagedRewardedSetResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.mixnet_contract(),
                &MixnetQueryMsg::GetRewardedSet {
                    limit: Some(9999),
                    start_after: None,
                },
            )
            .unwrap();
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

        // reward
        for (mix_id, _status) in &current_rewarded_set.nodes {
            self.app
                .execute_contract(
                    rewarding_validator(),
                    self.mixnet_contract(),
                    &MixnetExecuteMsg::RewardMixnode {
                        mix_id: *mix_id,
                        performance: Performance::hundred(),
                    },
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
        let new_rewarded_set = current_rewarded_set
            .nodes
            .into_iter()
            .enumerate()
            .map(|(i, (node, _))| {
                LayerAssignment::new(node, ((i as u8 % 3) + 1).try_into().unwrap())
            })
            .collect();

        self.app
            .execute_contract(
                rewarding_validator(),
                self.mixnet_contract(),
                &MixnetExecuteMsg::AdvanceCurrentEpoch {
                    new_rewarded_set,
                    expected_active_set_size: current_params.active_set_size,
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
        proxy: Option<Addr>,
        cost_params: MixNodeCostParams,
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
        let content =
            ContractMessageContent::new(Addr::unchecked(owner), proxy, vec![stake], payload);
        let sign_payload = SignableMixNodeBondingMsg::new(signing_nonce, content);
        let plaintext = sign_payload.to_plaintext().unwrap();
        let signature = keypair.private_key().sign(&plaintext);
        let msg_signature = MessageSignature::from(signature.to_bytes().as_ref());

        (mixnode, msg_signature)
    }
}

pub fn instantiate_contracts(
    mut initial_funds: HashMap<Addr, Vec<Coin>>,
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
            &nym_mixnet_contract_common::InstantiateMsg {
                rewarding_validator_address: REWARDING_VALIDATOR.to_string(),
                vesting_contract_address: "placeholder".to_string(),
                rewarding_denom: MIX_DENOM.to_string(),
                epochs_in_interval: 720,
                epoch_duration: Duration::from_secs(60 * 60),
                initial_rewarding_params: InitialRewardingParams {
                    initial_reward_pool: Decimal::from_atomics(250_000_000_000_000u128, 0).unwrap(),
                    initial_staking_supply: Decimal::from_atomics(223_000_000_000_000u128, 0)
                        .unwrap(),
                    staking_supply_scale_factor: Percent::hundred(),
                    sybil_resistance: Percent::from_percentage_value(30).unwrap(),
                    active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                    interval_pool_emission: Percent::from_percentage_value(2).unwrap(),
                    rewarded_set_size: 240,
                    active_set_size: 100,
                },
            },
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

fn mixnet_contract_wrapper() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            mixnet_contract::contract::execute,
            mixnet_contract::contract::instantiate,
            mixnet_contract::contract::query,
        )
        .with_migrate(mixnet_contract::contract::migrate),
    )
}

fn vesting_contract_wrapper() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            vesting_contract::contract::execute,
            vesting_contract::contract::instantiate,
            vesting_contract::contract::query,
        )
        .with_migrate(vesting_contract::contract::migrate),
    )
}
