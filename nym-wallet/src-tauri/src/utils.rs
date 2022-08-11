// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BackendError;
use crate::nymd_client;
use crate::state::WalletState;
use cosmwasm_std::Decimal;
use mixnet_contract_common::{IdentityKey, NodeId, Percent};
use nym_types::currency::DecCoin;
use nym_types::mixnode::MixNodeCostParams;
use nym_wallet_types::app::AppEnv;
use serde::{Deserialize, Serialize};
use validator_client::nymd::traits::MixnetQueryClient;
use validator_client::nymd::{tx, Coin, CosmosCoin, Gas, GasPrice};

fn get_env_as_option(key: &str) -> Option<String> {
    match ::std::env::var(key) {
        Ok(res) => Some(res),
        Err(_e) => None,
    }
}

#[tauri::command]
pub fn get_env() -> AppEnv {
    AppEnv {
        ADMIN_ADDRESS: get_env_as_option("ADMIN_ADDRESS"),
        SHOW_TERMINAL: get_env_as_option("SHOW_TERMINAL"),
        ENABLE_QA_MODE: get_env_as_option("ENABLE_QA_MODE"),
    }
}

#[tauri::command]
pub async fn owns_mixnode(state: tauri::State<'_, WalletState>) -> Result<bool, BackendError> {
    todo!()
    // Ok(nymd_client!(state)
    //     .owns_mixnode(nymd_client!(state).address())
    //     .await?
    //     .is_some())
}

#[tauri::command]
pub async fn owns_gateway(state: tauri::State<'_, WalletState>) -> Result<bool, BackendError> {
    todo!()
    // Ok(nymd_client!(state)
    //     .owns_gateway(nymd_client!(state).address())
    //     .await?
    //     .is_some())
}

#[tauri::command]
pub async fn try_convert_pubkey_to_mix_id(
    state: tauri::State<'_, WalletState>,
    mix_identity: IdentityKey,
) -> Result<Option<NodeId>, BackendError> {
    let res = nymd_client!(state)
        .deprecated_get_mixnode_details_by_identity(mix_identity)
        .await?;
    Ok(res.map(|mixnode_details| mixnode_details.mix_id()))
}

#[tauri::command]
pub async fn default_mixnode_cost_params(
    state: tauri::State<'_, WalletState>,
    profit_margin_percent: Percent,
) -> Result<MixNodeCostParams, BackendError> {
    // attaches the old pre-update default operating cost of 40 nym per interval
    let guard = state.read().await;

    // since this is only a temporary solution until users are required to provide their own cost
    // params, we can make the assumption that it's always safe to use the mix denom here
    let current_network = guard.current_network();
    let denom = current_network.mix_denom().display;

    Ok(MixNodeCostParams {
        profit_margin_percent,
        interval_operating_cost: DecCoin {
            denom: denom.into(),
            amount: Decimal::from_atomics(40u32, 0).unwrap(),
        },
    })
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Operation {
    Upload,
    Init,
    Migrate,
    ChangeAdmin,
    Send,

    BondMixnode,
    BondMixnodeOnBehalf,
    UnbondMixnode,
    UnbondMixnodeOnBehalf,
    UpdateMixnodeConfig,
    DelegateToMixnode,
    DelegateToMixnodeOnBehalf,
    UndelegateFromMixnode,
    UndelegateFromMixnodeOnBehalf,

    BondGateway,
    BondGatewayOnBehalf,
    UnbondGateway,
    UnbondGatewayOnBehalf,

    UpdateContractSettings,

    BeginMixnodeRewarding,
    FinishMixnodeRewarding,

    TrackUnbondGateway,
    TrackUnbondMixnode,
    WithdrawVestedCoins,
    TrackUndelegation,
    CreatePeriodicVestingAccount,

    AdvanceCurrentInterval,
    AdvanceCurrentEpoch,
    WriteRewardedSet,
    ClearRewardedSet,
    UpdateMixnetAddress,
    CheckpointMixnodes,
    ReconcileDelegations,
}

impl Operation {
    fn default_gas_limit(&self) -> Gas {
        match self {
            Operation::Upload => 3_000_000u64.into(),
            Operation::Init => 500_000u64.into(),
            Operation::Migrate => 200_000u64.into(),
            Operation::ChangeAdmin => 80_000u64.into(),
            Operation::Send => 80_000u64.into(),

            Operation::BondMixnode => 175_000u64.into(),
            Operation::BondMixnodeOnBehalf => 200_000u64.into(),
            Operation::UnbondMixnode => 175_000u64.into(),
            Operation::UnbondMixnodeOnBehalf => 175_000u64.into(),
            Operation::UpdateMixnodeConfig => 175_000u64.into(),
            Operation::DelegateToMixnode => 175_000u64.into(),
            Operation::DelegateToMixnodeOnBehalf => 175_000u64.into(),
            Operation::UndelegateFromMixnode => 175_000u64.into(),
            Operation::UndelegateFromMixnodeOnBehalf => 175_000u64.into(),

            Operation::BondGateway => 175_000u64.into(),
            Operation::BondGatewayOnBehalf => 200_000u64.into(),
            Operation::UnbondGateway => 175_000u64.into(),
            Operation::UnbondGatewayOnBehalf => 200_000u64.into(),

            Operation::UpdateContractSettings => 175_000u64.into(),
            Operation::BeginMixnodeRewarding => 175_000u64.into(),
            Operation::FinishMixnodeRewarding => 175_000u64.into(),
            Operation::TrackUnbondGateway => 175_000u64.into(),
            Operation::TrackUnbondMixnode => 175_000u64.into(),
            Operation::WithdrawVestedCoins => 175_000u64.into(),
            Operation::TrackUndelegation => 175_000u64.into(),
            Operation::CreatePeriodicVestingAccount => 175_000u64.into(),
            Operation::AdvanceCurrentInterval => 175_000u64.into(),
            Operation::WriteRewardedSet => 175_000u64.into(),
            Operation::ClearRewardedSet => 175_000u64.into(),
            Operation::UpdateMixnetAddress => 80_000u64.into(),
            Operation::CheckpointMixnodes => 175_000u64.into(),
            Operation::ReconcileDelegations => 500_000u64.into(),
            Operation::AdvanceCurrentEpoch => 175_000u64.into(),
        }
    }

    fn calculate_fee(gas_price: &GasPrice, gas_limit: Gas) -> CosmosCoin {
        gas_price * gas_limit
    }

    fn determine_custom_fee(gas_price: &GasPrice, gas_limit: Gas) -> tx::Fee {
        let fee = Self::calculate_fee(gas_price, gas_limit);
        tx::Fee::from_amount_and_gas(fee, gas_limit)
    }

    fn default_fee(&self, gas_price: &GasPrice) -> tx::Fee {
        Self::determine_custom_fee(gas_price, self.default_gas_limit())
    }
}

#[tauri::command]
pub async fn get_old_and_incorrect_hardcoded_fee(
    state: tauri::State<'_, WalletState>,
    operation: Operation,
) -> Result<DecCoin, BackendError> {
    let guard = state.read().await;
    let mut approximate_fee = operation.default_fee(guard.current_client()?.nymd.gas_price());
    // on all our chains it should only ever contain a single type of currency
    assert_eq!(approximate_fee.amount.len(), 1);
    let coin: Coin = approximate_fee.amount.pop().unwrap().into();
    log::info!("hardcoded fee for {:?} is {:?}", operation, coin);
    guard.attempt_convert_to_display_dec_coin(coin)
}
