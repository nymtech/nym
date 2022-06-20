use crate::error::BackendError;
use crate::nymd_client;
use crate::state::State;
use nym_types::currency::MajorCurrencyAmount;
use nym_wallet_types::app::AppEnv;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
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
pub async fn owns_mixnode(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
    Ok(nymd_client!(state)
        .owns_mixnode(nymd_client!(state).address())
        .await?
        .is_some())
}

#[tauri::command]
pub async fn owns_gateway(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<bool, BackendError> {
    Ok(nymd_client!(state)
        .owns_gateway(nymd_client!(state).address())
        .await?
        .is_some())
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
    state: tauri::State<'_, Arc<RwLock<State>>>,
    operation: Operation,
) -> Result<MajorCurrencyAmount, BackendError> {
    let mut approximate_fee = operation.default_fee(nymd_client!(state).gas_price());
    // on all our chains it should only ever contain a single type of currency
    assert_eq!(approximate_fee.amount.len(), 1);
    let coin: Coin = approximate_fee.amount.pop().unwrap().into();
    log::info!("hardcoded fee for {:?} is {:?}", operation, coin);
    Ok(coin.into())
}
